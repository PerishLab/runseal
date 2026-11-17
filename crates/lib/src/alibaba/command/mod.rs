mod args;

use super::{Auth, Client};
use crate::tool::Reply;
use anyhow::{Result, bail};
use args::{Deed, Input};
use clap::Parser;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    let input = parse(argv).map_err(|_| invalid())?;
    execute(input, vars).map(|value| Reply::plain("alibaba", value))
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let parsed = parse(argv);
    if let Err(error) = &parsed
        && matches!(
            error.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        )
    {
        print!("{error}");
        return Ok(());
    }
    execute(parsed.map_err(|_| invalid())?, vars)
        .map(|value| Reply::plain("alibaba", value))?
        .emit(argv)
}

fn parse(argv: &[String]) -> std::result::Result<Input, clap::Error> {
    Input::try_parse_from(std::iter::once("@alibaba".to_string()).chain(argv.iter().cloned()))
}

fn invalid() -> anyhow::Error {
    anyhow::anyhow!("invalid @alibaba command; use --help")
}

fn execute(input: Input, vars: &BTreeMap<String, String>) -> Result<Value> {
    let path = input
        .state
        .or_else(|| vars.get("ALIBABA_CLOUD_STATE_FILE").map(Into::into))
        .ok_or_else(|| anyhow::anyhow!("@alibaba requires --state or profile ALIBABA_CLOUD_STATE_FILE pointing to an absolute private state file"))?;
    if let Deed::Auth(deed) = input.deed {
        let held = Auth::new(&path)?;
        return match deed {
            args::Auth::Begin => held.begin(),
            args::Auth::Complete { response } => held.complete(&response),
            args::Auth::Refresh => held.refresh(),
            args::Auth::Renew => held.renew(),
            args::Auth::Recover => held.recover(),
            args::Auth::Status => held.status(),
            args::Auth::Logout => held.logout(),
        }
        .map_err(Into::into);
    }
    let account = input
        .account
        .as_deref()
        .or_else(|| vars.get("ALIBABA_CLOUD_ACCOUNT_ID").map(String::as_str))
        .ok_or_else(|| anyhow::anyhow!("Alibaba identity requires an explicit owning --account"))?;
    let principal = input
        .principal
        .as_deref()
        .or_else(|| vars.get("ALIBABA_CLOUD_PRINCIPAL_ID").map(String::as_str))
        .ok_or_else(|| {
            anyhow::anyhow!("Alibaba identity requires an explicit delegated --principal")
        })?;
    let width = (1..=256).contains(&principal.len());
    let lexical = principal.is_ascii() && !principal.bytes().any(|byte| byte.is_ascii_control());
    if !width || !lexical {
        bail!("Alibaba expected principal is malformed");
    }
    let identity = Client::open(&path, account)?.identity()?;
    if identity.principal != principal {
        bail!("Alibaba observed caller differs from the explicitly declared principal");
    }
    serde_json::to_value(identity).map_err(Into::into)
}
