use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use path_absolutize::Absolutize;

use crate::core::config::Config;

#[derive(Debug)]
pub(super) struct Listed {
    pub(super) name: String,
    pub(super) source: &'static str,
    pub(super) file: PathBuf,
}

pub(super) fn resolve(config: &Config, name: &str) -> Result<PathBuf> {
    let searched = search_paths(config, name);
    for candidate in &searched {
        if is_runnable(candidate) {
            return candidate
                .absolutize()
                .with_context(|| format!("failed to absolutize wrapper: {}", candidate.display()))
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

pub(super) fn effective(config: &Config) -> Result<Vec<Listed>> {
    let dirs = search_dirs(config);
    let mut names = BTreeSet::new();

    for dir in &dirs {
        collect_names(dir, &mut names)?;
    }

    let mut wrappers = Vec::new();
    for name in names {
        let file = resolve(config, &name)?;
        let source = source(&file, &dirs[0]);
        wrappers.push(Listed { name, source, file });
    }
    Ok(wrappers)
}

fn collect_names(dir: &Path, names: &mut BTreeSet<String>) -> Result<()> {
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
    if !is_runnable(&file) {
        return None;
    }
    listed_name(&file)
}

fn source(file: &Path, profile: &Path) -> &'static str {
    if file.starts_with(profile) {
        "profile"
    } else {
        "home"
    }
}

pub(super) fn path_env(config: &Config) -> Result<std::ffi::OsString> {
    env::join_paths(search_dirs(config)).context("failed to build RUNSEAL_WRAPPER_PATH")
}

pub(super) fn is_deno(path: &Path) -> bool {
    path.extension().and_then(std::ffi::OsStr::to_str) == Some("ts")
}

fn is_runnable(path: &Path) -> bool {
    if is_deno(path) {
        return path.is_file();
    }
    is_executable(path)
}

fn search_paths(config: &Config, name: &str) -> Vec<PathBuf> {
    search_dirs(config)
        .into_iter()
        .flat_map(|dir| candidates(&dir, name))
        .collect()
}

fn search_dirs(config: &Config) -> Vec<PathBuf> {
    vec![
        profile_root(&config.profile)
            .join(".runseal")
            .join("wrappers"),
        config.home.join("wrappers"),
    ]
}

fn profile_root(profile_path: &Path) -> &Path {
    profile_path.parent().unwrap_or(Path::new("."))
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

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
fn listed_name(path: &Path) -> Option<String> {
    if path.extension().and_then(std::ffi::OsStr::to_str) != Some("sh") {
        if path.extension().and_then(std::ffi::OsStr::to_str) == Some("ts") {
            let stem = path.file_stem()?.to_str()?;
            validate_symbol_name(stem).ok()?;
            return Some(stem.to_string());
        }
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    validate_symbol_name(stem).ok()?;
    Some(stem.to_string())
}

#[cfg(windows)]
fn listed_name(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    if let Some(ext) = path.extension().and_then(std::ffi::OsStr::to_str)
        && matches_ignore_ascii_case(ext, &["ts", "exe", "cmd", "bat"])
    {
        let stem = path.file_stem()?.to_str()?;
        validate_symbol_name(stem).ok()?;
        return Some(stem.to_string());
    }

    validate_symbol_name(file_name).ok()?;
    Some(file_name.to_string())
}

#[cfg(windows)]
fn matches_ignore_ascii_case(value: &str, expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn validate_symbol_name(name: &str) -> Result<()> {
    if name == "." || name == ".." {
        bail!("reserved name");
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("expected only ASCII letters, numbers, '.', '_', and '-'");
    }
    Ok(())
}
