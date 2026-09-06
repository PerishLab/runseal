mod deed;
mod help;

use std::collections::BTreeMap;
use std::process::{Command, Output};

use anyhow::{Context, Result, bail};
use serde_json::json;

use self::deed::{Admin, Deed, Token};
use super::Reply;
use crate::parse;

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    Seat::open(argv, vars)?.act("kubectl", argv)
}

pub(super) fn selected(argv: &[String]) -> Result<bool> {
    let line = parse::Line::take(argv)?;
    Ok(line.rest.first().is_some_and(|held| held == "admin"))
}

pub(super) fn help() {
    print!("{}", help::TEXT);
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let line = parse::Line::take(argv)?;
    if matches!(line.rest.as_slice(), [admin, resource, verb, ..] if admin == "admin" && resource == "token" && verb == "create")
    {
        bail!("@forgejo admin token create is available only to a structured caller");
    }
    call(argv, vars)?.emit(argv)
}

struct Seat<'a> {
    deed: Deed,
    vars: &'a BTreeMap<String, String>,
}

impl<'a> Seat<'a> {
    fn open(argv: &[String], vars: &'a BTreeMap<String, String>) -> Result<Self> {
        let line = parse::Line::take(argv)?;
        if line.watch {
            bail!("@forgejo admin does not support --watch");
        }
        let deed = Deed::take(&line.rest, line.flag("scopes"))?;
        Ok(Self { deed, vars })
    }

    fn act(&self, command: &str, argv: &[String]) -> Result<Reply> {
        match &self.deed {
            Deed::Admin(Admin::Token(Token::List { account })) => {
                let value = super::Seat::open(argv, self.vars)?.authority(account)?;
                Ok(Reply::plain("tokens", value))
            }
            Deed::Admin(Admin::Token(Token::Create {
                account,
                name,
                scopes,
            })) => self.create(command, account, name, scopes),
            Deed::Admin(Admin::Token(Token::Drop { account, name, id })) => {
                self.drop(command, account, name, *id)
            }
        }
    }

    fn create(&self, command: &str, account: &str, name: &str, scopes: &[String]) -> Result<Reply> {
        let scope = scopes.join(",");
        let namespace = required(self.vars, "RUNSEAL_FORGEJO_ISSUER_NAMESPACE")?;
        let workload = required(self.vars, "RUNSEAL_FORGEJO_ISSUER_WORKLOAD")?;
        let container = required(self.vars, "RUNSEAL_FORGEJO_ISSUER_CONTAINER")?;
        let output = self.execute(
            command,
            &[
                "-n",
                namespace,
                "exec",
                workload,
                "-c",
                container,
                "--",
                "forgejo",
                "admin",
                "user",
                "generate-access-token",
                "--username",
                account,
                "--token-name",
                name,
                "--raw",
                "--scopes",
                &scope,
            ],
            "token issuer",
        )?;
        let secret = text(output.stdout, "token issuer")?;
        if secret.len() <= 8 || secret.chars().any(char::is_whitespace) {
            bail!("Forgejo token issuer returned an invalid secret");
        }
        Ok(Reply::guarded(
            "token",
            json!({"account": account, "name": name, "scopes": scopes}),
            secret,
        ))
    }

    fn drop(&self, command: &str, account: &str, name: &str, id: u64) -> Result<Reply> {
        let identity = account.to_ascii_lowercase();
        let namespace = required(self.vars, "RUNSEAL_FORGEJO_STORE_NAMESPACE")?;
        let pod = required(self.vars, "RUNSEAL_FORGEJO_STORE_POD")?;
        let database = required(self.vars, "RUNSEAL_FORGEJO_STORE_DATABASE")?;
        let user = required(self.vars, "RUNSEAL_FORGEJO_STORE_USER")?;
        let statement = format!(
            "with removed as (delete from access_token where id = {id} and uid = (select id from \"user\" where lower_name = '{identity}') and name = '{name}' returning id) select count(*) from removed;"
        );
        let output = self.execute(
            command,
            &[
                "-n",
                namespace,
                "exec",
                pod,
                "--",
                "psql",
                "-U",
                user,
                "-d",
                database,
                "-v",
                "ON_ERROR_STOP=1",
                "-Atc",
                &statement,
            ],
            "token store",
        )?;
        let count = text(output.stdout, "token store")?;
        if count != "1" {
            bail!("Forgejo token store revoked {count} rows for {name}");
        }
        Ok(Reply::plain(
            "token",
            json!({"account": account, "name": name, "id": id, "revoked": true}),
        ))
    }

    fn execute(&self, command: &str, args: &[&str], label: &str) -> Result<Output> {
        let output = Command::new(command)
            .args(args)
            .envs(self.vars)
            .output()
            .with_context(|| format!("cannot invoke Forgejo {label}"))?;
        if !output.status.success() {
            bail!("Forgejo {label} failed ({})", output.status);
        }
        Ok(output)
    }
}

fn required<'a>(vars: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str> {
    vars.get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("@forgejo admin requires {name}"))
}

fn text(bytes: Vec<u8>, label: &str) -> Result<String> {
    let held =
        String::from_utf8(bytes).with_context(|| format!("Forgejo {label} returned non-UTF-8"))?;
    Ok(held.trim_end_matches(['\r', '\n']).to_string())
}
