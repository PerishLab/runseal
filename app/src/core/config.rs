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
        let path = Seek {
            name: name.as_deref(),
            cwd: &cwd,
            home: &initial,
        }
        .find()?;
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

struct Seek<'a> {
    name: Option<&'a str>,
    cwd: &'a Path,
    home: &'a Path,
}

impl Seek<'_> {
    fn find(self) -> Result<Option<PathBuf>> {
        if let Some(path) = self.walk()? {
            return Ok(Some(path));
        }
        if let Some(path) = self.held(self.flat())? {
            return Ok(Some(path));
        }
        if let Some(path) = self.held(self.nested())? {
            return Ok(Some(path));
        }
        self.missing()
    }

    fn walk(&self) -> Result<Option<PathBuf>> {
        let file = match self.name {
            None => "runseal.toml".to_string(),
            Some(name) => format!("runseal.{name}.toml"),
        };
        match plumb::config::discover(self.cwd, &file) {
            Ok(path) => absolute(&path, self.cwd, "profile file").map(Some),
            Err(_) => Ok(None),
        }
    }

    fn flat(&self) -> PathBuf {
        self.home.join("profiles").join(match self.name {
            None => "default.toml".to_string(),
            Some(name) => format!("{name}.toml"),
        })
    }

    fn nested(&self) -> PathBuf {
        let dir = self.name.unwrap_or("default");
        self.home.join("profiles").join(dir).join("runseal.toml")
    }

    fn held(&self, path: PathBuf) -> Result<Option<PathBuf>> {
        if path.is_file() {
            return absolute(&path, self.cwd, "profile file").map(Some);
        }
        Ok(None)
    }

    fn missing(&self) -> Result<Option<PathBuf>> {
        let Some(name) = self.name else {
            return Ok(None);
        };
        bail!(
            "named profile not found: :{name}; expected runseal.{name}.toml from {} upward, {}, or {}",
            self.cwd.display(),
            self.flat().display(),
            self.nested().display()
        );
    }
}

fn absolute(path: &Path, cwd: &Path, name: &str) -> Result<PathBuf> {
    path.absolutize_from(cwd)
        .with_context(|| format!("failed to absolutize {name}: {}", path.display()))
        .map(|path| path.to_path_buf())
}
