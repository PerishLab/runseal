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
        return crate::parse::write(&error.to_string());
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
            args::Auth::Federate { binding, home } => {
                let binding = binding.or_else(|| vars.get("ALIBABA_CLOUD_FEDERATION_FILE").map(Into::into)).ok_or_else(|| anyhow::anyhow!("Federate requires --binding-file or profile ALIBABA_CLOUD_FEDERATION_FILE"))?;
                let home = home.or_else(|| vars.get("ENSIGN_HOME").map(Into::into)).ok_or_else(|| anyhow::anyhow!("Federate requires --ensign-home or profile ENSIGN_HOME"))?;
                held.federate(&home,&bindingfile(&binding)?)
            },
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
        .or_else(|| vars.get("ALIBABA_CLOUD_PRINCIPAL_ID").map(String::as_str));
    if let Some(principal) = principal {
        let width = (1..=256).contains(&principal.len());
        let lexical =
            principal.is_ascii() && !principal.bytes().any(|byte| byte.is_ascii_control());
        if !width || !lexical {
            bail!("Alibaba expected principal is malformed");
        }
    }
    let mut client = Client::open(&path, account)?;
    if matches!(input.deed, Deed::Role) {
        let identity = client.role()?;
        if principal.is_some_and(|expected| identity.principal != expected) {
            bail!("Alibaba role caller differs from the declared principal");
        }
        return serde_json::to_value(identity).map_err(Into::into);
    }
    let identity = client.identity()?;
    if principal.is_some_and(|expected| identity.principal != expected) {
        bail!("Alibaba observed caller differs from the explicitly declared principal");
    }
    serde_json::to_value(identity).map_err(Into::into)
}

fn bindingfile(path: &std::path::Path) -> Result<super::Binding> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        bail!("Alibaba public federation binding exceeds its allowance");
    }
    let binding: super::Binding = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("Alibaba public federation binding JSON refused"))?;
    binding.validate()?;
    Ok(binding)
}
