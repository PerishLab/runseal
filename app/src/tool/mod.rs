use std::collections::BTreeMap;
use std::fmt;

use anyhow::{Result, bail};
use serde_json::Value;
use zeroize::Zeroize;

pub mod cloudflare;
pub mod forgejo;

#[derive(PartialEq)]
pub struct Reply {
    pub kind: &'static str,
    pub value: Value,
    secret: Option<Secret>,
}

#[derive(PartialEq)]
pub struct Secret(String);

impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reply")
            .field("kind", &self.kind)
            .field("value", &self.value)
            .field("secret", &self.secret)
            .finish()
    }
}

impl Reply {
    pub fn plain(kind: &'static str, value: Value) -> Self {
        Self {
            kind,
            value,
            secret: None,
        }
    }

    pub(crate) fn guarded(kind: &'static str, value: Value, secret: String) -> Self {
        Self {
            kind,
            value,
            secret: Some(Secret(secret)),
        }
    }

    pub fn secret(&self) -> Option<&Secret> {
        self.secret.as_ref()
    }

    pub(crate) fn emit(self, argv: &[String]) -> Result<()> {
        let line = crate::parse::Line::take(argv)?;
        if line.json {
            println!("{}", crate::parse::dump(self.kind, &self.value)?);
        } else {
            crate::parse::show(&self.value);
        }
        Ok(())
    }
}

pub fn call(name: &str, argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    match name {
        "cloudflare" => cloudflare::call(argv, vars),
        "forgejo" => forgejo::call(argv, vars),
        "forgejo-admin" => forgejo::admin::call(argv, vars),
        _ => bail!("unknown Runseal tool: @{name}"),
    }
}

pub fn run(name: &str, argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    match name {
        "cloudflare" => cloudflare::run(argv, vars),
        "forgejo" => forgejo::run(argv, vars),
        "forgejo-admin" => forgejo::admin::run(argv, vars),
        _ => bail!("unknown Runseal tool: @{name}"),
    }
}
