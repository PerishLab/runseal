use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, bail};

use super::options;

pub(super) fn required(args: &[String]) -> Result<String> {
    find(args)?.ok_or_else(|| {
        anyhow::anyhow!(
            "missing GitHub token: pass --token, --token-file, --token-env, or set GITHUB_TOKEN"
        )
    })
}

pub(super) fn optional(args: &[String]) -> Result<Option<String>> {
    find(args)
}

fn find(args: &[String]) -> Result<Option<String>> {
    if let Some(token) = options::optional(args, "--token").filter(|value| !value.is_empty()) {
        return Ok(Some(token));
    }
    if let Some(path) = options::optional(args, "--token-file") {
        let values = env(Path::new(&path))?;
        if let Some(token) = values.get("GITHUB_TOKEN").filter(|value| !value.is_empty()) {
            return Ok(Some(token.clone()));
        }
        bail!("GITHUB_TOKEN not set in {path}");
    }
    if let Some(name) = options::optional(args, "--token-env") {
        let token = std::env::var(&name)
            .with_context(|| format!("environment variable not set: {name}"))?;
        if token.is_empty() {
            bail!("environment variable is empty: {name}");
        }
        return Ok(Some(token));
    }
    Ok(std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|value| !value.is_empty()))
}

fn env(path: &Path) -> Result<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read token file: {}", path.display()))?;
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            bail!("invalid line in {}: {line}", path.display());
        };
        values.insert(
            key.trim().to_string(),
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string(),
        );
    }
    Ok(values)
}
