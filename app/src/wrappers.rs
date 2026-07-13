use crate::core::config::Config;
use crate::core::symbol;
use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;
use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub(super) struct Listed {
    pub(super) name: String,
    pub(super) source: &'static str,
    pub(super) file: PathBuf,
}
fn collect(dir: &Path, names: &mut BTreeSet<String>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read wrapper dir: {}", dir.display()))?;
        if let Some(name) = listed(entry) {
            names.insert(name);
        }
    }
    Ok(())
}
fn listed(entry: std::fs::DirEntry) -> Option<String> {
    let file = entry.path();
    if !(Bin { path: &file }).runnable() {
        return None;
    }
    (Bin { path: &file }).label()
}
fn source(file: &Path, profile: &Path) -> &'static str {
    if file.starts_with(profile) {
        "profile"
    } else {
        "home"
    }
}
fn root(profile: &Path) -> &Path {
    profile.parent().unwrap_or(Path::new("."))
}
#[cfg(unix)]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    if Path::new(name).extension().is_some() {
        return vec![dir.join(name)];
    }
    vec![
        dir.join(format!("{name}.ts")),
        dir.join(format!("{name}.sh")),
    ]
}
#[cfg(windows)]
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let exact = dir.join(name);
    if Path::new(name).extension().is_some() {
        return vec![exact];
    }
    [exact]
        .into_iter()
        .chain(
            ["ts", "exe", "cmd", "bat"]
                .into_iter()
                .map(|ext| dir.join(format!("{name}.{ext}"))),
        )
        .collect()
}
#[cfg(windows)]
fn matches(value: &str, expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

pub(super) struct Fleet<'a> {
    pub(super) config: &'a Config,
}

impl Fleet<'_> {
    pub(super) fn resolve(&self, name: &str) -> Result<PathBuf> {
        let searched = self.paths(name);
        for candidate in &searched {
            if (Bin { path: candidate }).runnable() {
                return candidate
                    .absolutize()
                    .with_context(|| {
                        format!("failed to absolutize wrapper: {}", candidate.display())
                    })
                    .map(|path| path.to_path_buf());
            }
        }

        let searched = searched
            .iter()
            .map(|path| format!("- {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("wrapper not found: :{name}\nsearched:\n{searched}")
    }

    pub(super) fn effective(&self) -> Result<Vec<Listed>> {
        let dirs = self.dirs();
        let mut names = BTreeSet::new();

        for dir in &dirs {
            collect(dir, &mut names)?;
        }

        let mut wrappers = Vec::new();
        for name in names {
            let file = self.resolve(&name)?;
            let source = source(&file, &dirs[0]);
            wrappers.push(Listed { name, source, file });
        }
        Ok(wrappers)
    }

    pub(super) fn env(&self) -> Result<std::ffi::OsString> {
        env::join_paths(self.dirs()).context("failed to build RUNSEAL_WRAPPER_PATH")
    }

    fn paths(&self, name: &str) -> Vec<PathBuf> {
        self.dirs()
            .into_iter()
            .flat_map(|dir| candidates(&dir, name))
            .collect()
    }

    fn dirs(&self) -> Vec<PathBuf> {
        vec![
            root(&self.config.profile).join(".runseal").join("wrappers"),
            self.config.home.join("wrappers"),
        ]
    }
}

pub(super) struct Bin<'a> {
    pub(super) path: &'a Path,
}

impl Bin<'_> {
    pub(super) fn deno(&self) -> bool {
        self.path.extension().and_then(std::ffi::OsStr::to_str) == Some("ts")
    }

    fn runnable(&self) -> bool {
        if self.deno() {
            return self.path.is_file();
        }
        self.executable()
    }

    #[cfg(unix)]
    fn executable(&self) -> bool {
        use std::os::unix::fs::PermissionsExt;

        self.path.is_file()
            && self
                .path
                .metadata()
                .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }

    #[cfg(windows)]
    fn executable(&self) -> bool {
        self.path.is_file()
    }

    #[cfg(unix)]
    fn label(&self) -> Option<String> {
        if self.path.extension().and_then(std::ffi::OsStr::to_str) != Some("sh") {
            if self.path.extension().and_then(std::ffi::OsStr::to_str) == Some("ts") {
                let stem = self.path.file_stem()?.to_str()?;
                symbol::valid(stem).ok()?;
                return Some(stem.to_string());
            }
            return None;
        }
        let stem = self.path.file_stem()?.to_str()?;
        symbol::valid(stem).ok()?;
        Some(stem.to_string())
    }

    #[cfg(windows)]
    fn label(&self) -> Option<String> {
        let file = self.path.file_name()?.to_str()?;
        if let Some(ext) = self.path.extension().and_then(std::ffi::OsStr::to_str)
            && matches(ext, &["ts", "exe", "cmd", "bat"])
        {
            let stem = self.path.file_stem()?.to_str()?;
            symbol::valid(stem).ok()?;
            return Some(stem.to_string());
        }

        symbol::valid(file).ok()?;
        Some(file.to_string())
    }
}
