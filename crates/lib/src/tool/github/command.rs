use super::{Cause, Client, Error};
use crate::{parse, tool::Reply};
use anyhow::Result;
use clap::{Parser, Subcommand, error::ErrorKind};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "@github",
    about = "Native GitHub operations on an explicitly selected credential"
)]
struct Input {
    #[arg(
        long = "token-file",
        global = true,
        help = "Absolute private file; otherwise profile GITHUB_TOKEN_FILE or GITHUB_TOKEN"
    )]
    file: Option<PathBuf>,
    #[arg(long, global = true, help = "Write structured provider receipts")]
    json: bool,
    #[command(subcommand)]
    deed: Deed,
}

#[derive(Subcommand)]
enum Deed {
    Repository { owner: String, name: String },
    Project { owner: String, number: u32 },
    Close { node: String },
}

pub(crate) fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    let input =
        Input::try_parse_from(std::iter::once("@github").chain(argv.iter().map(String::as_str)))?;
    Ok(Reply::plain("github", execute(input, vars)?))
}

pub(crate) fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let input = match Input::try_parse_from(
        std::iter::once("@github").chain(argv.iter().map(String::as_str)),
    ) {
        Ok(input) => input,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let json = input.json;
    match execute(input, vars) {
        Ok(value) => display(json, &value),
        Err(error) => {
            if json {
                println!("{}", parse::dump("error", &serde_json::to_value(&error)?)?);
            }
            Err(error.into())
        }
    }
}

fn display(json: bool, value: &Value) -> Result<()> {
    if json {
        println!("{}", parse::dump("github", value)?);
    } else {
        parse::show(value);
    }
    Ok(())
}

fn execute(input: Input, vars: &BTreeMap<String, String>) -> Result<Value, Error> {
    let file = input
        .file
        .or_else(|| vars.get("GITHUB_TOKEN_FILE").map(Into::into));
    let client = match file {
        Some(file) => Client::open(&file)?,
        None => Client::token(
            vars.get("GITHUB_TOKEN")
                .ok_or_else(|| Error::local(Cause::Custody))?,
        )?,
    };
    let value = match input.deed {
        Deed::Repository { owner, name } => serde_json::to_value(client.repository(&owner, &name)?),
        Deed::Project { owner, number } => serde_json::to_value(client.project(&owner, number)?),
        Deed::Close { node } => serde_json::to_value(client.close(&node)?),
    };
    value.map_err(|_| Error::local(Cause::Shape))
}
