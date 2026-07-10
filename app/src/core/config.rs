use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;

#[derive(Debug, Clone)]
pub struct Input {
    pub profile: Option<PathBuf>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub runseal: Option<PathBuf>,
    pub profile: Option<PathBuf>,
}

impl Env {
    pub fn process() -> Self {
        Self {
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| present(path)),
            runseal: std::env::var_os("RUNSEAL_HOME")
                .map(PathBuf::from)
                .filter(|path| present(path)),
            profile: std::env::var_os("RUNSEAL_PROFILE_HOME")
                .map(PathBuf::from)
                .filter(|path| present(path)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub profile: PathBuf,
    pub command: Vec<String>,
    pub home: PathBuf,
    pub profiles: PathBuf,
}

impl Config {
    pub fn build(input: Input, env: Env, cwd: &Path) -> Result<Self> {
        let home = absolute(&resolve(&env)?, cwd, "RUNSEAL_HOME")?;
        let profiles = env
            .profile
            .filter(|path| present(path))
            .unwrap_or_else(|| home.join("profiles"));
        let profiles = absolute(&profiles, cwd, "RUNSEAL_PROFILE_HOME")?;
        let profile = discover(input.profile, cwd, &profiles)?;

        Ok(Self {
            profile,
            command: input.command,
            home,
            profiles,
        })
    }
}

pub fn resolve(env: &Env) -> Result<PathBuf> {
    env.runseal
        .clone()
        .filter(|path| present(path))
        .or_else(|| {
            env.home
                .clone()
                .filter(|path| present(path))
                .map(|home| home.join(".runseal"))
        })
        .ok_or_else(|| anyhow::anyhow!("HOME is not set; pass --profile or set RUNSEAL_HOME"))
}

fn present(path: &Path) -> bool {
    !path.as_os_str().is_empty()
}

fn discover(explicit: Option<PathBuf>, cwd: &Path, profiles: &Path) -> Result<PathBuf> {
    if let Some(profile) = explicit {
        let profile = if profile.is_absolute() {
            profile
        } else {
            cwd.join(profile)
        };
        if !profile.is_file() {
            bail!("profile file not found: {}", profile.display());
        }
        return file(&profile);
    }

    let mut searched = Vec::new();
    for candidate in candidates(cwd, profiles) {
        if candidate.is_file() {
            return file(&candidate);
        }
        searched.push(candidate);
    }

    let searched = searched
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "no runseal profile found from {} upward and no default profile under {}.\nHint: create runseal.toml here, pass --profile <path>, or add {}/default.toml.\nSearched:\n{searched}",
        cwd.display(),
        profiles.display(),
        profiles.display()
    )
}

fn candidates(cwd: &Path, profiles: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in cwd.ancestors() {
        candidates.extend(
            extensions()
                .iter()
                .map(|ext| dir.join(format!("runseal.{ext}"))),
        );
    }
    candidates.extend(
        extensions()
            .iter()
            .map(|ext| profiles.join(format!("default.{ext}"))),
    );
    candidates
}

fn file(path: &Path) -> Result<PathBuf> {
    path.absolutize()
        .with_context(|| format!("failed to absolutize profile file: {}", path.display()))
        .map(|path| path.to_path_buf())
}

fn absolute(path: &Path, cwd: &Path, name: &str) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    path.absolutize()
        .with_context(|| format!("failed to absolutize {name}: {}", path.display()))
        .map(|path| path.to_path_buf())
}

pub fn extensions() -> &'static [&'static str] {
    &["toml", "yaml", "yml", "json"]
}
