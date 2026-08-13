use std::collections::BTreeMap;

use anyhow::{Result, bail};

pub mod forgejo;

pub fn run(name: &str, argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    match name {
        "forgejo" => forgejo::run(argv, vars),
        _ => bail!("unknown Runseal tool: @{name}"),
    }
}
