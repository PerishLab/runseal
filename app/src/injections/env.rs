use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::core::app::AppContext;
use crate::core::profile::{EnvOpProfile, EnvProfile};

pub(crate) struct Env {
    cfg: EnvProfile,
}

impl Env {
    pub(crate) fn new(cfg: EnvProfile) -> Self {
        Self { cfg }
    }

    pub(crate) fn name(&self) -> &'static str {
        "env"
    }

    pub(crate) fn validate(&self) -> Result<()> {
        for key in self.cfg.vars.keys() {
            Check::key(key)?;
        }
        for op in &self.cfg.ops {
            Check::op(op)?;
        }
        Ok(())
    }

    pub(crate) fn register(&mut self) -> Result<()> {
        Ok(())
    }

    pub(crate) fn export(&self, app: &dyn AppContext) -> Result<Vec<(String, String)>> {
        let mut env: BTreeMap<String, String> = self
            .cfg
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Editor { app, env: &mut env }.apply(&self.cfg.ops);
        Ok(env.into_iter().collect())
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

struct Check;

impl Check {
    fn op(op: &EnvOpProfile) -> Result<()> {
        match op {
            EnvOpProfile::Set { key, value } | EnvOpProfile::SetIfAbsent { key, value } => {
                Self::value(key, value)
            }
            EnvOpProfile::Prepend {
                key,
                value,
                separator,
                ..
            }
            | EnvOpProfile::Append {
                key,
                value,
                separator,
                ..
            } => Self::merge(key, value, separator),
            EnvOpProfile::Unset { key } => Self::key(key),
        }
    }

    fn key(key: &str) -> Result<()> {
        if key.trim().is_empty() {
            bail!("env var key must not be empty");
        }
        Ok(())
    }

    fn value(key: &str, value: &str) -> Result<()> {
        Self::key(key)?;
        if value.trim().is_empty() {
            bail!("env var value must not be empty");
        }
        Ok(())
    }

    fn merge(key: &str, value: &str, separator: &Option<String>) -> Result<()> {
        Self::value(key, value)?;
        if matches!(separator.as_deref(), Some("")) {
            bail!("separator must not be empty");
        }
        Ok(())
    }
}

struct Editor<'a> {
    app: &'a dyn AppContext,
    env: &'a mut BTreeMap<String, String>,
}

impl Editor<'_> {
    fn apply(&mut self, ops: &[EnvOpProfile]) {
        for op in ops {
            self.op(op);
        }
    }

    fn op(&mut self, op: &EnvOpProfile) {
        match op {
            EnvOpProfile::Set { key, value } => self.set(key, value),
            EnvOpProfile::SetIfAbsent { key, value } => self.absent(key, value),
            EnvOpProfile::Prepend {
                key,
                value,
                separator,
                dedup,
            } => self.merge(key, value, separator, *dedup, true),
            EnvOpProfile::Append {
                key,
                value,
                separator,
                dedup,
            } => self.merge(key, value, separator, *dedup, false),
            EnvOpProfile::Unset { key } => self.unset(key),
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        self.env.insert(key.to_string(), value.to_string());
    }

    fn absent(&mut self, key: &str, value: &str) {
        if !self.env.contains_key(key) && self.app.env().var(key).is_none() {
            self.set(key, value);
        }
    }

    fn merge(
        &mut self,
        key: &str,
        value: &str,
        delimiter: &Option<String>,
        dedup: bool,
        prepend: bool,
    ) {
        let delimiter = separator(delimiter);
        let base = self
            .env
            .get(key)
            .cloned()
            .or_else(|| self.app.env().var(key))
            .unwrap_or_default();
        let merged = if prepend {
            merge(value, &base, delimiter, dedup)
        } else {
            merge(&base, value, delimiter, dedup)
        };
        self.env.insert(key.to_string(), merged);
    }

    fn unset(&mut self, key: &str) {
        self.env.remove(key);
    }
}

fn separator(value: &Option<String>) -> &str {
    match value.as_deref() {
        None | Some("os") => os(),
        Some(custom) => custom,
    }
}

fn os() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

fn merge(left: &str, right: &str, separator: &str, dedup: bool) -> String {
    let mut out = Vec::new();
    out.extend(split(left, separator));
    out.extend(split(right, separator));

    if dedup {
        return unique(out).join(separator);
    }
    out.join(separator)
}

fn unique(entries: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        push(&mut out, entry);
    }
    out
}

fn push(out: &mut Vec<String>, entry: String) {
    if !out.contains(&entry) {
        out.push(entry);
    }
}

fn split(value: &str, separator: &str) -> Vec<String> {
    value
        .split(separator)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}
