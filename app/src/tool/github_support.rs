use std::{collections::BTreeMap, path::Path, process::Command, time::Duration};

use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const CORE_REPOS: &[&str] = &["PerishCode/runseal", "PerishCode/sidecar"];

pub(super) struct Request;

pub(super) struct Body;

pub(super) struct Token;

pub(super) struct Prefix;

pub(super) struct Repo;

pub(super) struct Branch;

pub(super) struct Options;

struct Env;

impl Request {
    pub(super) fn text(
        method: &str,
        path: &str,
        token: &str,
        body: String,
    ) -> Result<Option<String>> {
        Self::send(
            method,
            path,
            Some(token),
            Some(serde_json::json!({
                "body": body,
            })),
        )
        .map(|payload| {
            Some(serde_json::to_string(&payload).expect("GitHub payload should serialize"))
        })
    }

    pub(super) fn send(
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<JsonValue>,
    ) -> Result<JsonValue> {
        let base = std::env::var("RUNSEAL_GITHUB_API_BASE")
            .unwrap_or_else(|_| GITHUB_API_BASE.to_string());
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let url = format!("{base}{path}");
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let method = method
            .parse::<reqwest::Method>()
            .with_context(|| format!("invalid HTTP method: {method}"))?;
        let mut request = client
            .request(method.clone(), &url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(reqwest::header::USER_AGENT, "runseal");
        if let Some(token) = token.filter(|value| !value.is_empty()) {
            request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .json(&body);
        }
        let response = request
            .send()
            .with_context(|| format!("GitHub API {method} {path} unreachable"))?;
        let status = response.status();
        let raw = response
            .text()
            .with_context(|| format!("GitHub API {method} {path} returned unreadable body"))?;
        if !status.is_success() {
            bail!("GitHub API {method} {path} -> {}: {raw}", status.as_u16());
        }
        if raw.trim().is_empty() {
            return Ok(JsonValue::Object(Default::default()));
        }
        serde_json::from_str(&raw)
            .with_context(|| format!("GitHub API returned invalid JSON for {path}"))
    }
}

impl Body {
    pub(super) fn read(args: &[String]) -> Result<String> {
        let inline = Options::optional(args, "--body");
        let file = Options::optional(args, "--body-file");
        match (inline, file) {
            (Some(_), Some(_)) => bail!("pass exactly one of --body or --body-file"),
            (None, None) => bail!("pass exactly one of --body or --body-file"),
            (Some(body), None) => Ok(body),
            (None, Some(path)) => std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read body file: {path}")),
        }
    }

    pub(super) fn validate(args: &[String], body: &str, default: usize) -> Result<()> {
        let max = Self::max(args, default)?;
        if max == 0 {
            return Ok(());
        }
        let count = body.chars().count();
        if count > max {
            bail!("body length {count} exceeds --body-max={max}");
        }
        Ok(())
    }

    fn max(args: &[String], default: usize) -> Result<usize> {
        let Some(value) = Options::optional(args, "--body-max") else {
            return Ok(default);
        };
        value
            .parse::<usize>()
            .with_context(|| format!("invalid --body-max: {value}"))
    }
}

impl Token {
    pub(super) fn required(args: &[String]) -> Result<String> {
        Self::find(args)?.ok_or_else(|| {
            anyhow::anyhow!(
                "missing GitHub token: pass --token, --token-file, --token-env, or set GITHUB_TOKEN"
            )
        })
    }

    pub(super) fn optional(args: &[String]) -> Result<Option<String>> {
        Self::find(args)
    }

    fn find(args: &[String]) -> Result<Option<String>> {
        if let Some(token) = Options::optional(args, "--token").filter(|value| !value.is_empty()) {
            return Ok(Some(token));
        }
        if let Some(path) = Options::optional(args, "--token-file") {
            let values = Env::parse(Path::new(&path))?;
            if let Some(token) = values.get("GITHUB_TOKEN").filter(|value| !value.is_empty()) {
                return Ok(Some(token.clone()));
            }
            bail!("GITHUB_TOKEN not set in {path}");
        }
        if let Some(name) = Options::optional(args, "--token-env") {
            let token = std::env::var(&name)
                .with_context(|| format!("environment variable not set: {name}"))?;
            if token.is_empty() {
                bail!("environment variable is empty: {name}");
            }
            return Ok(Some(token));
        }
        Ok(std::env::var("GITHUB_TOKEN")
            .ok()
            .filter(|value| !value.is_empty()))
    }
}

impl Prefix {
    pub(super) fn enabled(args: &[String]) -> Result<bool> {
        Options::boolean(args, "--prefix-enable").map(|value| value.unwrap_or(false))
    }
}

impl Repo {
    pub(super) fn current() -> Result<String> {
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
        Self::parse(url.trim())
    }

    pub(super) fn github() -> Result<String> {
        let repo = Self::current()?;
        if repo.contains('/') {
            return Ok(repo);
        }
        bail!("cannot parse GitHub owner/repo from current repository")
    }

    pub(super) fn core(repo: &str) -> bool {
        CORE_REPOS
            .iter()
            .any(|value| value.eq_ignore_ascii_case(repo))
    }

    fn parse(url: &str) -> Result<String> {
        if let Ok(repo) = Self::hosted(
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
        if let Ok(repo) = Self::hosted(
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
}

impl Branch {
    pub(super) fn current() -> Result<String> {
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
}

impl Options {
    pub(super) fn required(args: &[String], name: &str) -> Result<String> {
        Self::optional(args, name).ok_or_else(|| anyhow::anyhow!("{name} is required"))
    }

    fn boolean(args: &[String], name: &str) -> Result<Option<bool>> {
        let prefix = format!("{name}=");
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if arg == name {
                return Ok(Some(true));
            }
            if let Some(value) = arg.strip_prefix(&prefix) {
                return Self::parse(name, value).map(Some);
            }
            index += 1;
        }
        Ok(None)
    }

    fn parse(name: &str, value: &str) -> Result<bool> {
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => bail!("{name} expects true or false"),
        }
    }

    fn optional(args: &[String], name: &str) -> Option<String> {
        let prefix = format!("{name}=");
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if arg == name {
                return args.get(index + 1).cloned();
            }
            if let Some(value) = arg.strip_prefix(&prefix) {
                return Some(value.to_string());
            }
            index += 1;
        }
        None
    }
}

impl Env {
    fn parse(path: &Path) -> Result<BTreeMap<String, String>> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read token file: {}", path.display()))?;
        let mut values = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                bail!("invalid line in {}: {line}", path.display());
            };
            values.insert(
                key.trim().to_string(),
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
        Ok(values)
    }
}
