use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde_json::Value;

pub mod forgejo;

#[derive(Debug, PartialEq)]
pub struct Reply {
    pub kind: &'static str,
    pub value: Value,
}

pub fn call(name: &str, argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    match name {
        "forgejo" => forgejo::call(argv, vars),
        _ => bail!("unknown Runseal tool: @{name}"),
    }
}

pub fn run(name: &str, argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    match name {
        "forgejo" => forgejo::run(argv, vars),
        _ => bail!("unknown Runseal tool: @{name}"),
    }
}
