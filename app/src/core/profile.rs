use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use serde::Deserialize;

fn default_enabled() -> bool {
    true
}

fn default_cleanup() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub resources: Option<ResourcesProfile>,
    #[serde(default)]
    pub deno: Option<DenoProfile>,
    #[serde(default)]
    pub injections: Vec<InjectionProfile>,
}

#[derive(Debug, Deserialize)]
struct ResourceMetadata {
    #[serde(default)]
    resources: Option<ResourcesProfile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResourcesProfile {
    pub root: PathBuf,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DenoProfile {
    #[serde(default)]
    pub config: Option<PathBuf>,
    #[serde(default)]
    pub lock: Option<PathBuf>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InjectionProfile {
    Env(EnvProfile),
    Symlink(SymlinkProfile),
    Argv(ArgvProfile),
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnvProfile {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub ops: Vec<EnvOpProfile>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EnvOpProfile {
    Set {
        key: String,
        value: String,
    },
    SetIfAbsent {
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

impl EnvOpProfile {
    pub fn key(&self) -> &str {
        match self {
            Self::Set { key, .. }
            | Self::SetIfAbsent { key, .. }
            | Self::Prepend { key, .. }
            | Self::Append { key, .. }
            | Self::Unset { key } => key,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SymlinkProfile {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub source: PathBuf,
    pub target: PathBuf,
    #[serde(default)]
    pub on_exist: SymlinkOnExist,
    #[serde(default = "default_cleanup")]
    pub cleanup: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArgvProfile {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum SymlinkOnExist {
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
    normalize_deno_paths(path, &mut profile)?;
    normalize_symlink_paths(path, &mut profile)?;
    let resources = profile.resources.clone();
    normalize_env_resource_values(path, resources.as_ref(), &mut profile)?;
    Ok(profile)
}

pub fn load_resources(path: &Path) -> Result<Option<ResourcesProfile>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read profile file: {}", path.display()))?;
    let metadata: ResourceMetadata = match path.extension().and_then(|ext| ext.to_str()) {
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

pub fn resolve_resource_uri(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
    uri: &str,
) -> Result<PathBuf> {
    let relative = parse_resource_uri(uri)?;
    resolve_resource_root(profile_path, resources)?
        .join(relative)
        .absolutize()
        .with_context(|| format!("failed to absolutize resource URI: {uri}"))
        .map(|path| path.to_path_buf())
}

pub fn resolve_resource_root(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
) -> Result<PathBuf> {
    let resources = resources.ok_or_else(|| {
        anyhow::anyhow!(
            "resource root is not configured in {}; add [resources] root = \".local\"",
            profile_path.display()
        )
    })?;
    if resources.root.as_os_str().is_empty() {
        anyhow::bail!(
            "resources.root must not be empty in {}",
            profile_path.display()
        );
    }
    normalize_path(
        &resources.root,
        profile_path.parent().unwrap_or(Path::new(".")),
    )
}

fn normalize_symlink_paths(profile_path: &Path, profile: &mut Profile) -> Result<()> {
    let base_dir = profile_path.parent().unwrap_or(Path::new("."));
    for injection in &mut profile.injections {
        if let InjectionProfile::Symlink(spec) = injection {
            spec.source = normalize_path(&spec.source, base_dir)?;
            spec.target = normalize_path(&spec.target, base_dir)?;
        }
    }
    Ok(())
}

fn normalize_deno_paths(profile_path: &Path, profile: &mut Profile) -> Result<()> {
    let Some(deno) = &mut profile.deno else {
        return Ok(());
    };
    let base_dir = profile_path.parent().unwrap_or(Path::new("."));
    if let Some(config) = &mut deno.config {
        *config = normalize_path(config, base_dir)?;
    }
    if let Some(lock) = &mut deno.lock {
        *lock = normalize_path(lock, base_dir)?;
    }
    Ok(())
}

fn normalize_env_resource_values(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
    profile: &mut Profile,
) -> Result<()> {
    for injection in &mut profile.injections {
        normalize_injection(profile_path, resources, injection)?;
    }
    Ok(())
}

fn normalize_injection(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
    injection: &mut InjectionProfile,
) -> Result<()> {
    let InjectionProfile::Env(spec) = injection else {
        return Ok(());
    };
    for value in spec.vars.values_mut() {
        normalize_env_value(profile_path, resources, value)?;
    }
    for op in &mut spec.ops {
        normalize_op(profile_path, resources, op)?;
    }
    Ok(())
}

fn normalize_op(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
    op: &mut EnvOpProfile,
) -> Result<()> {
    match op {
        EnvOpProfile::Set { value, .. }
        | EnvOpProfile::SetIfAbsent { value, .. }
        | EnvOpProfile::Prepend { value, .. }
        | EnvOpProfile::Append { value, .. } => normalize_env_value(profile_path, resources, value),
        EnvOpProfile::Unset { .. } => Ok(()),
    }
}

fn normalize_env_value(
    profile_path: &Path,
    resources: Option<&ResourcesProfile>,
    value: &mut String,
) -> Result<()> {
    if !value.starts_with("resource://") {
        return Ok(());
    }
    *value = resolve_resource_uri(profile_path, resources, value)?
        .to_string_lossy()
        .into_owned();
    Ok(())
}

fn parse_resource_uri(uri: &str) -> Result<PathBuf> {
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

fn normalize_path(path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    let expanded = shellexpand::tilde(&raw);
    let expanded_path = PathBuf::from(expanded.as_ref());
    if expanded_path.is_absolute() {
        return Ok(expanded_path);
    }
    Ok(expanded_path.absolutize_from(base_dir)?.to_path_buf())
}
