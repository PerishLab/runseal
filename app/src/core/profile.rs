use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use serde::Deserialize;

fn enabled() -> bool {
    true
}

fn cleanup() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub resources: Option<Resources>,
    #[serde(default)]
    pub deno: Option<Deno>,
    #[serde(default)]
    pub injections: Vec<Injection>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    #[serde(default)]
    resources: Option<Resources>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Resources {
    pub root: PathBuf,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Deno {
    #[serde(default)]
    pub config: Option<PathBuf>,
    #[serde(default)]
    pub lock: Option<PathBuf>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Injection {
    Env(Env),
    Symlink(Symlink),
    Argv(Argv),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Env {
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub ops: Vec<Op>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Set {
        key: String,
        value: String,
    },
    #[serde(rename = "set_if_absent")]
    Absent {
        key: String,
        value: String,
    },
    Prepend {
        key: String,
        value: String,
        #[serde(default)]
        separator: Option<String>,
        #[serde(default)]
        dedup: bool,
    },
    Append {
        key: String,
        value: String,
        #[serde(default)]
        separator: Option<String>,
        #[serde(default)]
        dedup: bool,
    },
    Unset {
        key: String,
    },
}

impl Op {
    pub fn key(&self) -> &str {
        match self {
            Self::Set { key, .. }
            | Self::Absent { key, .. }
            | Self::Prepend { key, .. }
            | Self::Append { key, .. }
            | Self::Unset { key } => key,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Symlink {
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub source: PathBuf,
    pub target: PathBuf,
    #[serde(default, rename = "on_exist")]
    pub existing: Existing,
    #[serde(default = "cleanup")]
    pub cleanup: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Argv {
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum Existing {
    #[default]
    Error,
    Replace,
}

pub fn load(path: &Path) -> Result<Profile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read profile file: {}", path.display()))?;
    let mut profile: Profile = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML: {}", path.display()))?,
        Some("yaml") | Some("yml") => yaml_serde::from_str(&raw)
            .with_context(|| format!("failed to parse YAML: {}", path.display()))?,
        Some("json") => serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse JSON: {}", path.display()))?,
        _ => anyhow::bail!(
            "unsupported profile format: {} (expected .toml, .yaml, .yml, or .json)",
            path.display()
        ),
    };
    deno(path, &mut profile)?;
    symlinks(path, &mut profile)?;
    let resources = profile.resources.clone();
    env(path, resources.as_ref(), &mut profile)?;
    Ok(profile)
}

pub fn resources(path: &Path) -> Result<Option<Resources>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read profile file: {}", path.display()))?;
    let metadata: Metadata = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML: {}", path.display()))?,
        Some("yaml") | Some("yml") => yaml_serde::from_str(&raw)
            .with_context(|| format!("failed to parse YAML: {}", path.display()))?,
        Some("json") => serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse JSON: {}", path.display()))?,
        _ => anyhow::bail!(
            "unsupported profile format: {} (expected .toml, .yaml, .yml, or .json)",
            path.display()
        ),
    };
    Ok(metadata.resources)
}

pub fn resolve(profile: &Path, resources: Option<&Resources>, uri: &str) -> Result<PathBuf> {
    let relative = parse(uri)?;
    root(profile, resources)?
        .join(relative)
        .absolutize()
        .with_context(|| format!("failed to absolutize resource URI: {uri}"))
        .map(|path| path.to_path_buf())
}

pub fn root(profile: &Path, resources: Option<&Resources>) -> Result<PathBuf> {
    let resources = resources.ok_or_else(|| {
        anyhow::anyhow!(
            "resource root is not configured in {}; add [resources] root = \".local\"",
            profile.display()
        )
    })?;
    if resources.root.as_os_str().is_empty() {
        anyhow::bail!("resources.root must not be empty in {}", profile.display());
    }
    normalize(&resources.root, profile.parent().unwrap_or(Path::new(".")))
}

fn symlinks(path: &Path, profile: &mut Profile) -> Result<()> {
    let base = path.parent().unwrap_or(Path::new("."));
    for injection in &mut profile.injections {
        if let Injection::Symlink(spec) = injection {
            spec.source = normalize(&spec.source, base)?;
            spec.target = normalize(&spec.target, base)?;
        }
    }
    Ok(())
}

fn deno(path: &Path, profile: &mut Profile) -> Result<()> {
    let Some(deno) = &mut profile.deno else {
        return Ok(());
    };
    let base = path.parent().unwrap_or(Path::new("."));
    if let Some(config) = &mut deno.config {
        *config = normalize(config, base)?;
    }
    if let Some(lock) = &mut deno.lock {
        *lock = normalize(lock, base)?;
    }
    Ok(())
}

fn env(path: &Path, resources: Option<&Resources>, profile: &mut Profile) -> Result<()> {
    for injection in &mut profile.injections {
        self::injection(path, resources, injection)?;
    }
    Ok(())
}

fn injection(path: &Path, resources: Option<&Resources>, injection: &mut Injection) -> Result<()> {
    let Injection::Env(spec) = injection else {
        return Ok(());
    };
    for value in spec.vars.values_mut() {
        self::value(path, resources, value)?;
    }
    for op in &mut spec.ops {
        self::op(path, resources, op)?;
    }
    Ok(())
}

fn op(path: &Path, resources: Option<&Resources>, op: &mut Op) -> Result<()> {
    match op {
        Op::Set { value, .. }
        | Op::Absent { value, .. }
        | Op::Prepend { value, .. }
        | Op::Append { value, .. } => self::value(path, resources, value),
        Op::Unset { .. } => Ok(()),
    }
}

fn value(path: &Path, resources: Option<&Resources>, value: &mut String) -> Result<()> {
    if !value.starts_with("resource://") {
        return Ok(());
    }
    *value = resolve(path, resources, value)?
        .to_string_lossy()
        .into_owned();
    Ok(())
}

fn parse(uri: &str) -> Result<PathBuf> {
    let Some(raw) = uri.strip_prefix("resource://") else {
        anyhow::bail!("expected resource URI to start with resource://");
    };
    if raw.is_empty() || raw == "." {
        return Ok(PathBuf::new());
    }
    if raw.contains('\\') {
        anyhow::bail!("resource URI path must use '/' separators");
    }

    let mut path = PathBuf::new();
    for segment in raw.split('/') {
        if segment.is_empty() {
            anyhow::bail!("resource URI path segment must not be empty");
        }
        if segment == "." || segment == ".." {
            anyhow::bail!("resource URI path must not contain '.' or '..'");
        }
        if segment.contains(':') {
            anyhow::bail!("resource URI path segment must not contain ':'");
        }
        path.push(segment);
    }
    Ok(path)
}

fn normalize(path: &Path, base: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    let expanded = shellexpand::tilde(&raw);
    let expanded = PathBuf::from(expanded.as_ref());
    if expanded.is_absolute() {
        return Ok(expanded);
    }
    Ok(expanded.absolutize_from(base)?.to_path_buf())
}
