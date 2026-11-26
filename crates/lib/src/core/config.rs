use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;
use radicula_config::section;

use super::symbol;

const MANIFEST: &str = "plumb.toml";
const SECTION: &str = "runseal";
const STANDALONE: &str = "runseal.toml";

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
        let initial = home(&cwd)?;
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

    pub fn home(cwd: &Path) -> Result<PathBuf> {
        let cwd = absolute(cwd, cwd, "current directory")?;
        home(&cwd)
    }

    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or("default")
    }

    pub fn manifest(&self) -> bool {
        self.name.is_none()
            && self
                .path
                .as_deref()
                .and_then(Path::file_name)
                .is_some_and(|name| name == OsStr::new(MANIFEST))
    }
}

fn home(cwd: &Path) -> Result<PathBuf> {
    let selected = std::env::var_os("RUNSEAL_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(data)
        .unwrap_or_else(|| PathBuf::from(".runseal"));
    absolute(&selected, cwd, "RUNSEAL_HOME")
}

fn data() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|path| path.join("runseal"))
    } else {
        std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|path| path.join(".runseal"))
    }
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
        match self.name {
            None => self.default(),
            Some(name) => self.named(name),
        }
    }

    fn named(&self, name: &str) -> Result<Option<PathBuf>> {
        let file = format!("runseal.{name}.toml");
        for root in self.cwd.ancestors() {
            let path = root.join(&file);
            if path.is_file() {
                return absolute(&path, self.cwd, "profile file").map(Some);
            }
        }
        Ok(None)
    }

    fn default(&self) -> Result<Option<PathBuf>> {
        for dir in self.cwd.ancestors() {
            if let Some(path) = self.inspect(dir)? {
                return absolute(&path, self.cwd, "profile file").map(Some);
            }
        }
        Ok(None)
    }

    fn inspect(&self, dir: &Path) -> Result<Option<PathBuf>> {
        let standalone = dir.join(STANDALONE);
        let manifest = dir.join(MANIFEST);
        let direct = standalone.is_file();
        if manifest.is_file() {
            let present = section::<toml::Table>(&manifest, SECTION)?.is_some();
            if present && direct {
                bail!(
                    "conflicting default profile in {}: both {MANIFEST} [{SECTION}] and {STANDALONE} exist",
                    dir.display()
                );
            }
            if !direct {
                return Ok(Some(manifest));
            }
        }
        if direct {
            return Ok(Some(standalone));
        }
        Ok(None)
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
