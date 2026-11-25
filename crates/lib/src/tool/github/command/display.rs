use super::Input;
use crate::github::{Cause, Client, Error};
use crate::parse;
use anyhow::Result;
use clap::{Parser, error::ErrorKind};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let input = match Input::try_parse_from(
        std::iter::once("@github").chain(argv.iter().map(String::as_str)),
    ) {
        Ok(input) => input,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            return parse::write(&error.to_string());
        }
        Err(error) => return Err(error.into()),
    };
    let json = input.json;
    match execute(input, vars) {
        Ok(value) => display(json, &value),
        Err(error) => {
            if json {
                parse::emit(&parse::dump("error", &serde_json::to_value(&error)?)?)?;
            }
            Err(error.into())
        }
    }
}

fn display(json: bool, value: &Value) -> Result<()> {
    if json {
        parse::emit(&parse::dump("github", value)?)
    } else {
        parse::show(value)
    }
}

pub(super) fn execute(input: Input, vars: &BTreeMap<String, String>) -> Result<Value, Error> {
    let client = if input.stdin {
        Client::reader(std::io::stdin().lock())?
    } else if let Some(file) = input
        .file
        .or_else(|| vars.get("GITHUB_TOKEN_FILE").map(Into::into))
    {
        Client::open(&file)?
    } else {
        Client::token(
            vars.get("GITHUB_TOKEN")
                .ok_or_else(|| Error::local(Cause::Custody))?,
        )?
    };
    super::apply(&client, input.deed)
}
