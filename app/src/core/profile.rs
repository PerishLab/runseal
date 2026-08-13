use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;
use serde::Deserialize;

use super::config::Config;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub env: Env,
    pub argv: BTreeMap<String, Vec<String>>,
    pub symlink: Vec<Symlink>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Env {
    pub vars: BTreeMap<String, String>,
    pub unset: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Profile {
    pub fn load(config: &Config) -> Result<Self> {
        let mut profile = match config.path.as_deref() {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .with_context(|| format!("cannot read {}", path.display()))?;
                toml::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))?
            }
            None => Self::default(),
        };
        profile.normalize(&config.root)?;
        Ok(profile)
    }

    fn normalize(&mut self, root: &Path) -> Result<()> {
        for held in self.env.vars.values_mut() {
            *held = value(root, held)?;
        }
        for args in self.argv.values_mut() {
            for held in args {
                *held = value(root, held)?;
            }
        }
        for link in &mut self.symlink {
            link.source = seat(&link.source, root, "symlink source")?;
            link.target = seat(&link.target, root, "symlink target")?;
        }
        Ok(())
    }
}

pub fn resolve(root: &Path, uri: &str) -> Result<PathBuf> {
    let (seat, raw) = if let Some(raw) = uri.strip_prefix("resource://") {
        (root.join(".runseal/resources"), raw)
    } else if let Some(raw) = uri.strip_prefix("local://") {
        (root.join(".local"), raw)
    } else {
        bail!("expected a resource:// or local:// URI");
    };
    let relative = parse(raw)?;
    absolute(&seat.join(relative), root, "profile URI")
}

fn value(root: &Path, held: &str) -> Result<String> {
    if !held.starts_with("resource://") && !held.starts_with("local://") {
        return Ok(held.to_string());
    }
    Ok(resolve(root, held)?.to_string_lossy().into_owned())
}

fn seat(path: &Path, root: &Path, name: &str) -> Result<PathBuf> {
    match path.to_str() {
        Some(uri) if uri.starts_with("resource://") || uri.starts_with("local://") => {
            resolve(root, uri)
        }
        _ => absolute(path, root, name),
    }
}

fn parse(raw: &str) -> Result<PathBuf> {
    if raw.is_empty() || raw == "." {
        return Ok(PathBuf::new());
    }
    if raw.contains('\\') {
        bail!("profile URI path must use '/' separators");
    }

    let mut path = PathBuf::new();
    for segment in raw.split('/') {
        if segment.is_empty() {
            bail!("profile URI path segment must not be empty");
        }
        if segment == "." || segment == ".." {
            bail!("profile URI path must not contain '.' or '..'");
        }
        if segment.contains(':') {
            bail!("profile URI path segment must not contain ':'");
        }
        path.push(segment);
    }
    Ok(path)
}

fn absolute(path: &Path, root: &Path, name: &str) -> Result<PathBuf> {
    path.absolutize_from(root)
        .with_context(|| format!("failed to absolutize {name}: {}", path.display()))
        .map(|path| path.to_path_buf())
}
