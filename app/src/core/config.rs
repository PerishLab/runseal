use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;

use super::symbol;

#[derive(Debug, Clone, Default, plumb::config::Cascade)]
struct Settings {
    home: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub command: Vec<String>,
    pub home: PathBuf,
    pub name: Option<String>,
    pub path: Option<PathBuf>,
    pub root: PathBuf,
}

impl Config {
    pub fn build(name: Option<String>, command: Vec<String>, cwd: &Path) -> Result<Self> {
        if let Some(name) = name.as_deref() {
            symbol::valid(name).with_context(|| format!("invalid profile name: :{name}"))?;
        }
        let cwd = absolute(cwd, cwd, "current directory")?;
        let base = Settings::resolve(None).context("unable to resolve Runseal configuration")?;
        let initial = home(&base, &cwd)?;
        let path = discover(name.as_deref(), &cwd, &initial)?;
        let root = path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.clone());

        Ok(Self {
            command,
            home: initial,
            name,
            path,
            root,
        })
    }

    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or("default")
    }
}

fn home(settings: &Settings, cwd: &Path) -> Result<PathBuf> {
    let selected = if settings.home.as_os_str().is_empty() {
        plumb::config::data("runseal").unwrap_or_else(|| PathBuf::from(".runseal"))
    } else {
        settings.home.clone()
    };
    absolute(&selected, cwd, "RUNSEAL_HOME")
}

fn discover(name: Option<&str>, cwd: &Path, home: &Path) -> Result<Option<PathBuf>> {
    let file = match name {
        None => "runseal.toml".to_string(),
        Some(name) => format!("runseal.{name}.toml"),
    };

    if let Ok(path) = plumb::config::discover(cwd, &file) {
        return absolute(&path, cwd, "profile file").map(Some);
    }

    let fallback = home.join("profiles").join(match name {
        None => "default.toml".to_string(),
        Some(name) => format!("{name}.toml"),
    });
    if fallback.is_file() {
        return absolute(&fallback, cwd, "profile file").map(Some);
    }

    if let Some(name) = name {
        bail!(
            "named profile not found: :{name}; expected {file} from {} upward or {}",
            cwd.display(),
            fallback.display()
        );
    }
    Ok(None)
}

fn absolute(path: &Path, cwd: &Path, name: &str) -> Result<PathBuf> {
    path.absolutize_from(cwd)
        .with_context(|| format!("failed to absolutize {name}: {}", path.display()))
        .map(|path| path.to_path_buf())
}
