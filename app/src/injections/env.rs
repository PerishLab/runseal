use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::core::app::AppContext;
use crate::core::profile::{EnvOpProfile, EnvProfile};

pub(crate) struct EnvInjection {
    cfg: EnvProfile,
}

impl EnvInjection {
    pub(crate) fn new(cfg: EnvProfile) -> Self {
        Self { cfg }
    }

    pub(crate) fn name(&self) -> &'static str {
        "env"
    }

    pub(crate) fn validate(&self) -> Result<()> {
        for key in self.cfg.vars.keys() {
            validate_key(key)?;
        }
        for op in &self.cfg.ops {
            validate_op(op)?;
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
        apply_ops(app, &mut env, &self.cfg.ops)?;
        Ok(env.into_iter().collect())
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

fn validate_op(op: &EnvOpProfile) -> Result<()> {
    match op {
        EnvOpProfile::Set { key, value } | EnvOpProfile::SetIfAbsent { key, value } => {
            validate_key_value(key, value)
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
        } => validate_merge(key, value, separator),
        EnvOpProfile::Unset { key } => validate_key(key),
    }
}

fn validate_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        bail!("env var key must not be empty");
    }
    Ok(())
}

fn validate_key_value(key: &str, value: &str) -> Result<()> {
    validate_key(key)?;
    if value.trim().is_empty() {
        bail!("env var value must not be empty");
    }
    Ok(())
}

fn validate_merge(key: &str, value: &str, separator: &Option<String>) -> Result<()> {
    validate_key_value(key, value)?;
    validate_separator(separator)
}

fn validate_separator(separator: &Option<String>) -> Result<()> {
    if matches!(separator.as_deref(), Some("")) {
        bail!("separator must not be empty");
    }
    Ok(())
}

fn apply_ops(
    app: &dyn AppContext,
    env: &mut BTreeMap<String, String>,
    ops: &[EnvOpProfile],
) -> Result<()> {
    for op in ops {
        apply_op(app, env, op);
    }
    Ok(())
}

fn apply_op(app: &dyn AppContext, env: &mut BTreeMap<String, String>, op: &EnvOpProfile) {
    match op {
        EnvOpProfile::Set { key, value } => set(env, key, value),
        EnvOpProfile::SetIfAbsent { key, value } => set_absent(app, env, key, value),
        EnvOpProfile::Prepend {
            key,
            value,
            separator,
            dedup,
        } => set_merged(app, env, key, value, separator, *dedup, true),
        EnvOpProfile::Append {
            key,
            value,
            separator,
            dedup,
        } => set_merged(app, env, key, value, separator, *dedup, false),
        EnvOpProfile::Unset { key } => unset(env, key),
    }
}

fn set(env: &mut BTreeMap<String, String>, key: &str, value: &str) {
    env.insert(key.to_string(), value.to_string());
}

fn set_absent(app: &dyn AppContext, env: &mut BTreeMap<String, String>, key: &str, value: &str) {
    if !env.contains_key(key) && app.env().var(key).is_none() {
        set(env, key, value);
    }
}

fn set_merged(
    app: &dyn AppContext,
    env: &mut BTreeMap<String, String>,
    key: &str,
    value: &str,
    separator: &Option<String>,
    dedup: bool,
    prepend: bool,
) {
    let merged = merge_env_op(app, env, key, value, separator, dedup, prepend);
    env.insert(key.to_string(), merged);
}

fn unset(env: &mut BTreeMap<String, String>, key: &str) {
    env.remove(key);
}

fn merge_env_op(
    app: &dyn AppContext,
    env: &BTreeMap<String, String>,
    key: &str,
    value: &str,
    separator: &Option<String>,
    dedup: bool,
    prepend: bool,
) -> String {
    let sep = separator_value(separator);
    let base = env
        .get(key)
        .cloned()
        .or_else(|| app.env().var(key))
        .unwrap_or_default();
    if prepend {
        merge_values(value, &base, sep, dedup)
    } else {
        merge_values(&base, value, sep, dedup)
    }
}

fn separator_value(separator: &Option<String>) -> &str {
    match separator.as_deref() {
        None | Some("os") => os_separator(),
        Some(custom) => custom,
    }
}

fn os_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

fn merge_values(left: &str, right: &str, separator: &str, dedup: bool) -> String {
    let mut out = Vec::new();
    let left_parts = split_parts(left, separator);
    let right_parts = split_parts(right, separator);

    out.extend(left_parts);
    out.extend(right_parts);

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

fn split_parts(value: &str, separator: &str) -> Vec<String> {
    value
        .split(separator)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}
