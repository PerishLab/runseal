use std::process::Command;

use anyhow::{Context, Result, bail};

const CORE: &[&str] = &["PerishCode/runseal", "PerishCode/sidecar"];

pub(super) fn repo() -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .with_context(|| "failed to execute command: git")?;
    if !output.status.success() {
        bail!(
            "git remote get-url origin failed with status {}",
            output.status.code().unwrap_or(1)
        );
    }
    let url = String::from_utf8(output.stdout)
        .with_context(|| "git remote get-url origin returned non-UTF-8 output")?;
    parse(url.trim())
}

pub(super) fn github() -> Result<String> {
    let target = repo()?;
    if target.contains('/') {
        return Ok(target);
    }
    bail!("cannot parse GitHub owner/repo from current repository")
}

pub(super) fn core(repo: &str) -> bool {
    CORE.iter().any(|value| value.eq_ignore_ascii_case(repo))
}

pub(super) fn branch() -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .with_context(|| "failed to execute command: git")?;
    if !output.status.success() {
        bail!(
            "git branch --show-current failed with status {}",
            output.status.code().unwrap_or(1)
        );
    }
    let branch = String::from_utf8(output.stdout)
        .with_context(|| "git branch --show-current returned non-UTF-8 output")?;
    let branch = branch.trim();
    if branch.is_empty() {
        bail!("git branch --show-current returned an empty branch name");
    }
    Ok(branch.to_string())
}

fn parse(url: &str) -> Result<String> {
    if let Ok(repo) = hosted(
        url,
        &[
            "git@github.com:",
            "ssh://git@github.com/",
            "https://github.com/",
            "http://github.com/",
        ],
    ) {
        return Ok(repo);
    }
    if let Ok(repo) = hosted(
        url,
        &[
            "git@gitee.com:",
            "ssh://git@gitee.com/",
            "https://gitee.com/",
            "http://gitee.com/",
        ],
    ) {
        return Ok(repo);
    }
    bail!("cannot parse owner/repo from origin url: {url}");
}

fn hosted(url: &str, prefixes: &[&str]) -> Result<String> {
    let Some(tail) = url
        .strip_prefix(prefixes[0])
        .or_else(|| url.strip_prefix(prefixes[1]))
        .or_else(|| url.strip_prefix(prefixes[2]))
        .or_else(|| url.strip_prefix(prefixes[3]))
    else {
        bail!("unmatched host");
    };
    let path = tail.trim_end_matches(".git");
    let mut parts = path.split('/');
    let Some(owner) = parts.next().filter(|value| !value.is_empty()) else {
        bail!("cannot parse owner/repo from origin url: {url}");
    };
    let Some(repo) = parts.next().filter(|value| !value.is_empty()) else {
        bail!("cannot parse owner/repo from origin url: {url}");
    };
    if parts.next().is_some() {
        bail!("cannot parse owner/repo from origin url: {url}");
    }
    Ok(format!("{owner}/{repo}"))
}
