use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use self::{api::Client, deed::Deed, secret::Reserved};
use super::Reply;
use crate::parse;

pub mod api;
mod control;
mod deed;
mod help;
mod secret;
mod token;

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    invoke(argv, vars, None)
}

pub fn invoke(
    argv: &[String],
    vars: &BTreeMap<String, String>,
    body: Option<Value>,
) -> Result<Reply> {
    Seat::open(argv, vars, body)?.act()
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    if argv.iter().any(|arg| arg == "--help") {
        print!("{}", help::TEXT);
        return Ok(());
    }
    let seat = Seat::open(argv, vars, None)?;
    let reserved = if seat.deed.secret() {
        let path = seat
            .line
            .flag("value-file")
            .context("@cloudflare token create and roll require --value-file")?;
        Some(Reserved::open(path, seat.deed.rolls())?)
    } else {
        None
    };
    let mut reply = seat.act()?;
    if let Some(reserved) = reserved {
        reserved.commit(&mut reply)?;
    }
    reply.emit(argv)
}

struct Seat {
    line: parse::Line,
    deed: Deed,
    account: Option<String>,
    body: Option<Value>,
    client: Client,
}

impl Seat {
    fn open(argv: &[String], vars: &BTreeMap<String, String>, body: Option<Value>) -> Result<Self> {
        let line = parse::Line::take(argv)?;
        if line.watch {
            bail!("@cloudflare does not support --watch");
        }
        let deed = Deed::take(&line.rest)?;
        let account = line
            .flag("account")
            .map(str::to_string)
            .or_else(|| vars.get("CLOUDFLARE_ACCOUNT_ID").cloned())
            .filter(|value| !value.is_empty());
        let base = line
            .url
            .clone()
            .or_else(|| vars.get("CLOUDFLARE_API_URL").cloned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://api.cloudflare.com/client/v4".into());
        let headers = auth(&line, vars)?;
        Ok(Self {
            line,
            deed,
            account,
            body,
            client: Client::new(base.trim_end_matches('/').to_string(), headers),
        })
    }

    fn act(&self) -> Result<Reply> {
        match &self.deed {
            deed::Deed::Token(token) => token::act(self, &self.client, token),
            deed::Deed::Control(control) => control::act(self, &self.client, control),
        }
    }

    fn account(&self) -> Result<String> {
        let account = self
            .account
            .as_deref()
            .context("@cloudflare account operations require --account or CLOUDFLARE_ACCOUNT_ID")?;
        Ok(format!("/accounts/{account}"))
    }

    fn body(&self) -> Result<Value> {
        if let Some(body) = &self.body {
            if !body.is_object() {
                bail!("@cloudflare token body must be a JSON object");
            }
            return Ok(body.clone());
        }
        let path = self
            .line
            .flag("body-file")
            .context("@cloudflare operation requires --body-file")?;
        api::object(path)
    }
}

fn auth(line: &parse::Line, vars: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    let token = if let Some(path) = line.file.as_deref().filter(|value| !value.is_empty()) {
        Some(load(path)?)
    } else if let Some(path) = vars
        .get("CLOUDFLARE_API_TOKEN_FILE")
        .filter(|value| !value.is_empty())
    {
        Some(load(path)?)
    } else {
        vars.get("CLOUDFLARE_API_TOKEN")
            .filter(|value| !value.is_empty())
            .cloned()
    };
    if let Some(token) = token {
        return Ok(BTreeMap::from([(
            "Authorization".into(),
            format!("Bearer {token}"),
        )]));
    }
    let key = if let Some(path) = vars
        .get("CLOUDFLARE_API_KEY_FILE")
        .filter(|value| !value.is_empty())
    {
        Some(load(path)?)
    } else {
        vars.get("CLOUDFLARE_API_KEY")
            .filter(|value| !value.is_empty())
            .cloned()
    };
    let email = vars
        .get("CLOUDFLARE_API_EMAIL")
        .filter(|value| !value.is_empty());
    match (key, email) {
        (Some(key), Some(email)) => Ok(BTreeMap::from([
            ("X-Auth-Email".into(), email.clone()),
            ("X-Auth-Key".into(), key),
        ])),
        _ => bail!(
            "@cloudflare requires an API token, or CLOUDFLARE_API_KEY with CLOUDFLARE_API_EMAIL"
        ),
    }
}

fn load(path: &str) -> Result<String> {
    let held = fs::read_to_string(path)
        .with_context(|| format!("unable to read Cloudflare token file {path}"))?;
    let token = held.trim();
    if token.is_empty() {
        bail!("Cloudflare token file is empty: {path}");
    }
    Ok(token.to_string())
}
