mod auth;
mod client;
mod command;
mod fault;
mod message;
mod model;
mod store;
mod wire;

pub use client::Client;
pub use fault::{Class, Error, Outcome};
pub use message::{Content, Message, Operation, Reply, Sender};
pub use model::{Identity, Receipt, Status};

use crate::tool::Reply as Output;
use anyhow::Result;
use clap::Parser;
use std::collections::BTreeMap;

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Output> {
    let input = command::Input::try_parse_from(
        std::iter::once("@feishu".to_string()).chain(argv.iter().cloned()),
    )
    .map_err(|_| anyhow::anyhow!("invalid @feishu command; use --help"))?;
    command::execute(input, vars)
        .map(|value| Output::plain("feishu", value))
        .map_err(Into::into)
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let parsed = command::Input::try_parse_from(
        std::iter::once("@feishu".to_string()).chain(argv.iter().cloned()),
    );
    if let Err(error) = &parsed
        && matches!(error.kind(), clap::error::ErrorKind::DisplayHelp)
    {
        print!("{error}");
        return Ok(());
    }
    let input = parsed.map_err(|_| anyhow::anyhow!("invalid @feishu command; use --help"))?;
    let json = input.json;
    let value = match command::execute(input, vars) {
        Ok(value) => value,
        Err(error) => {
            report(&error, json)?;
            return Err(error.into());
        }
    };
    if json {
        println!("{}", crate::parse::dump("feishu", &value)?);
    } else {
        crate::parse::show(&value);
    }
    Ok(())
}

fn report(error: &Error, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            crate::parse::dump("feishu", &serde_json::json!({"error":error}))?
        );
    }
    Ok(())
}
