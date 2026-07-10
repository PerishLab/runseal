use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::{Method, StatusCode, blocking::Client as Http};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Tea {
    #[serde(default)]
    logins: Vec<Login>,
}

#[derive(Clone, Deserialize)]
struct Login {
    name: String,
    url: String,
    token: String,
    #[serde(default)]
    active: bool,
}

pub(super) struct Client {
    base: String,
    token: String,
    http: Http,
}

impl Client {
    pub(super) fn load(args: &[String]) -> Result<Self> {
        let token = env("FORGEJO_TOKEN");
        let base = env("RUNSEAL_FORGEJO_API_BASE")
            .or_else(|| env("FORGEJO_URL"))
            .or_else(|| env("GITHUB_SERVER_URL"));
        let login = if token.is_none() || base.is_none() {
            Some(login(args)?)
        } else {
            None
        };
        let token = token
            .or_else(|| login.as_ref().map(|value| value.token.clone()))
            .filter(|value| !value.is_empty())
            .context("missing Forgejo token: configure Tea or set FORGEJO_TOKEN")?;
        let base = base
            .or_else(|| login.map(|value| value.url))
            .map(|value| api(&value))
            .context("missing Forgejo URL: configure Tea or set FORGEJO_URL")?;
        let http = Http::builder().timeout(Duration::from_secs(30)).build()?;
        Ok(Self { base, token, http })
    }

    pub(super) fn get(&self, path: &str, query: &[(String, String)]) -> Result<Value> {
        self.send(Method::GET, path, query, None)
    }

    pub(super) fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        let (status, value) = self.request(method.clone(), path, query, body)?;
        if !status.is_success() {
            bail!(
                "Forgejo API {method} {path} -> {}: {value}",
                status.as_u16()
            );
        }
        Ok(value)
    }

    pub(super) fn probe(&self, path: &str) -> Result<Option<Value>> {
        let (status, value) = self.request(Method::GET, path, &[], None)?;
        match status {
            StatusCode::NOT_FOUND => Ok(None),
            status if status.is_success() => Ok(Some(value)),
            _ => bail!("Forgejo API GET {path} -> {}: {value}", status.as_u16()),
        }
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<(StatusCode, Value)> {
        let mut url = reqwest::Url::parse(&format!("{}{}", self.base, route(path)))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        let mut request = self
            .http
            .request(method, url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("token {}", self.token),
            )
            .header(reqwest::header::USER_AGENT, "runseal");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().context("Forgejo API unreachable")?;
        let status = response.status();
        let raw = response
            .text()
            .context("Forgejo API returned unreadable body")?;
        Ok((status, parse(&raw)?))
    }
}

fn login(args: &[String]) -> Result<Login> {
    let path = config()?;
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read Tea config: {}", path.display()))?;
    let tea: Tea = yaml_serde::from_str(&raw).context("invalid Tea config")?;
    let name = option(args, "--login").or_else(|| env("FORGEJO_LOGIN"));
    if let Some(name) = name {
        return tea
            .logins
            .into_iter()
            .find(|login| login.name == name)
            .with_context(|| format!("Tea login not found: {name}"));
    }
    if let Some(login) = tea.logins.iter().find(|login| login.active) {
        return Ok(login.clone());
    }
    match tea.logins.as_slice() {
        [login] => Ok(login.clone()),
        [] => bail!("Tea config has no logins"),
        _ => bail!("multiple Tea logins found: pass --login or set FORGEJO_LOGIN"),
    }
}

fn config() -> Result<PathBuf> {
    if let Some(path) = env("RUNSEAL_TEA_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".tea/tea.yml"))
}

fn api(value: &str) -> String {
    let value = value.trim_end_matches('/');
    if value.ends_with("/api/v1") {
        value.to_string()
    } else {
        format!("{value}/api/v1")
    }
}

fn route(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn parse(raw: &str) -> Result<Value> {
    if raw.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(raw).context("Forgejo API returned invalid JSON")
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn option(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for (index, arg) in args.iter().enumerate() {
        if arg == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}
