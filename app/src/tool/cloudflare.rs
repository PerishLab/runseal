use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

use super::{dns_record, redirect_rule};

#[derive(Debug, Clone)]
pub(super) struct Config {
    account: Account,
    token: String,
    zone: Zone,
    manage: Manage,
}

#[derive(Debug, Clone)]
struct Account {
    id: String,
}

#[derive(Debug, Clone)]
struct Zone {
    name: String,
}

#[derive(Debug, Clone)]
struct Manage {
    host: String,
    origin: String,
    prefix: String,
}

struct Ruleset;

struct Bucket;

struct Env(BTreeMap<String, String>);

struct Options {
    query: Vec<(String, String)>,
    body: Option<JsonValue>,
}

pub fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "config" => config(args),
        "api" => api(args),
        "zone" => zone(args),
        "account" => account(args),
        "redirect-rule" => redirect_rule::eval(args),
        _ => bail!("unknown tool command: cloudflare {command}"),
    }
}

fn config(args: &[String]) -> Result<Option<String>> {
    match args {
        [command, key] if command == "get" => Config::fetch(key),
        [command] if command == "json" => Config::dump(),
        _ => bail!("usage: runseal @tool cloudflare config get <key>|json"),
    }
}

impl Config {
    fn fetch(key: &str) -> Result<Option<String>> {
        let config = Self::load()?;
        let value = match key {
            "account_id" => config.account.id,
            "zone_name" => config.zone.name,
            "manage_host" => config.manage.host,
            "manage_origin_host" => config.manage.origin,
            "manage_redirect_prefix" => config.manage.prefix,
            other => bail!("unknown Cloudflare config key: {other}"),
        };
        Ok(Some(value))
    }

    fn dump() -> Result<Option<String>> {
        let config = Self::load()?;
        Ok(Some(serde_json::to_string(&serde_json::json!({
            "account_id": config.account.id,
            "zone_name": config.zone.name,
            "manage_host": config.manage.host,
            "manage_origin_host": config.manage.origin,
            "manage_redirect_prefix": config.manage.prefix,
        }))?))
    }
}

fn api(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!(
            "usage: runseal @tool cloudflare api request <method> <path> [--query k=v]... [--json <json>]"
        );
    };
    if command != "request" {
        bail!(
            "usage: runseal @tool cloudflare api request <method> <path> [--query k=v]... [--json <json>]"
        );
    }
    let [method, path, options @ ..] = rest else {
        bail!(
            "usage: runseal @tool cloudflare api request <method> <path> [--query k=v]... [--json <json>]"
        );
    };
    let parsed = Options::parse(options)?;
    let config = Config::load()?;
    request(&config, method, path, parsed.query, parsed.body)
}

fn zone(args: &[String]) -> Result<Option<String>> {
    match args {
        [command, rest @ ..] if command == "get" => Zone::get(rest),
        [ruleset, rest @ ..] if ruleset == "ruleset" => Ruleset::eval(rest),
        [dns, command, rest @ ..] if dns == "dns-record" => dns_record::eval(command, rest),
        _ => bail!("usage: runseal @tool cloudflare zone get|ruleset|dns-record ..."),
    }
}

impl Zone {
    fn get(args: &[String]) -> Result<Option<String>> {
        let name = required(args, "--name")?;
        let config = Config::load()?;
        let payload = request(
            &config,
            "GET",
            "/zones",
            vec![("name".to_string(), name.clone())],
            None,
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        let result = value
            .get("result")
            .and_then(JsonValue::as_array)
            .context("Cloudflare zone response missing result array")?;
        if result.is_empty() {
            bail!("zone not found for name: {name}");
        }
        if result.len() != 1 {
            bail!("expected one zone for {name}, found {}", result.len());
        }
        Ok(Some(serde_json::to_string(&result[0])?))
    }
}

impl Ruleset {
    fn eval(args: &[String]) -> Result<Option<String>> {
        match args {
            [command, rest @ ..] if command == "list" => Self::list(rest),
            [command, rest @ ..] if command == "get" => Self::get(rest),
            [command, rest @ ..] if command == "create" => Self::create(rest),
            [rule, rest @ ..] if rule == "rule" => Self::rule(rest),
            _ => bail!("usage: runseal @tool cloudflare zone ruleset list|get|create|rule ..."),
        }
    }

    fn list(args: &[String]) -> Result<Option<String>> {
        let zone = required(args, "--zone-id")?;
        let config = Config::load()?;
        let payload = request(
            &config,
            "GET",
            &format!("/zones/{zone}/rulesets"),
            Vec::new(),
            None,
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Array(Vec::new())),
        )?))
    }

    fn get(args: &[String]) -> Result<Option<String>> {
        let zone = required(args, "--zone-id")?;
        let ruleset = required(args, "--ruleset-id")?;
        let config = Config::load()?;
        let payload = request(
            &config,
            "GET",
            &format!("/zones/{zone}/rulesets/{ruleset}"),
            Vec::new(),
            None,
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Null),
        )?))
    }

    fn create(args: &[String]) -> Result<Option<String>> {
        let zone = required(args, "--zone-id")?;
        let phase = required(args, "--phase")?;
        let name = required(args, "--name")?;
        let body = serde_json::json!({
            "kind": "zone",
            "name": name,
            "phase": phase,
            "rules": [],
        });
        let config = Config::load()?;
        let payload = request(
            &config,
            "POST",
            &format!("/zones/{zone}/rulesets"),
            Vec::new(),
            Some(body),
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Null),
        )?))
    }

    fn rule(args: &[String]) -> Result<Option<String>> {
        match args {
            [command, rest @ ..] if command == "add" => Self::change("POST", rest, None),
            [command, rest @ ..] if command == "update" => Self::update(rest),
            _ => bail!("usage: runseal @tool cloudflare zone ruleset rule add|update ..."),
        }
    }

    fn update(rest: &[String]) -> Result<Option<String>> {
        let rule = required(rest, "--rule-id")?;
        Self::change("PATCH", rest, Some(rule))
    }

    fn change(method: &str, rest: &[String], rule: Option<String>) -> Result<Option<String>> {
        let zone = required(rest, "--zone-id")?;
        let ruleset = required(rest, "--ruleset-id")?;
        let payload = required(rest, "--json")?;
        let body: JsonValue = serde_json::from_str(&payload).context("invalid rule JSON")?;
        let path = Self::route(&zone, &ruleset, rule);
        let config = Config::load()?;
        let payload = request(&config, method, &path, Vec::new(), Some(body))?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Null),
        )?))
    }

    fn route(zone: &str, ruleset: &str, rule: Option<String>) -> String {
        match rule {
            Some(id) => format!("/zones/{zone}/rulesets/{ruleset}/rules/{id}"),
            None => format!("/zones/{zone}/rulesets/{ruleset}/rules"),
        }
    }
}

fn account(args: &[String]) -> Result<Option<String>> {
    match args {
        [command, rest @ ..] if command == "get" => Account::get(rest),
        [r2, bucket, command, rest @ ..]
            if r2 == "r2" && bucket == "bucket" && command == "list" =>
        {
            Bucket::list(rest)
        }
        _ => bail!("usage: runseal @tool cloudflare account get|r2 bucket list ..."),
    }
}

impl Account {
    fn get(args: &[String]) -> Result<Option<String>> {
        let account = required(args, "--account-id")?;
        let config = Config::load()?;
        let payload = request(
            &config,
            "GET",
            &format!("/accounts/{account}"),
            Vec::new(),
            None,
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Null),
        )?))
    }
}

impl Bucket {
    fn list(args: &[String]) -> Result<Option<String>> {
        let account = required(args, "--account-id")?;
        let config = Config::load()?;
        let payload = request(
            &config,
            "GET",
            &format!("/accounts/{account}/r2/buckets"),
            Vec::new(),
            None,
        )?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value
                .get("result")
                .and_then(|result| result.get("buckets"))
                .unwrap_or(&JsonValue::Array(Vec::new())),
        )?))
    }
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut query = Vec::new();
        let mut body = None;
        let mut index = 0;
        while index < args.len() {
            index += parse(&args[index..], &mut query, &mut body)?;
        }
        Ok(Self { query, body })
    }
}

fn parse(
    args: &[String],
    query: &mut Vec<(String, String)>,
    body: &mut Option<JsonValue>,
) -> Result<usize> {
    match args.first().map(String::as_str) {
        Some("--query") => query.push(pair(need(args, "--query")?)?),
        Some("--json") => {
            *body = Some(
                serde_json::from_str(need(args, "--json")?).context("invalid --json payload")?,
            )
        }
        Some(other) => bail!("unknown Cloudflare option: {other}"),
        None => bail!("missing Cloudflare option"),
    }
    Ok(2)
}

fn need<'a>(args: &'a [String], name: &str) -> Result<&'a str> {
    args.get(1)
        .map(String::as_str)
        .with_context(|| format!("{name} requires a value"))
}

fn pair(value: &str) -> Result<(String, String)> {
    let Some((key, value)) = value.split_once('=') else {
        bail!("invalid --query value: {value}; expected key=value");
    };
    Ok((key.to_string(), value.to_string()))
}

impl Config {
    pub(super) fn load() -> Result<Self> {
        let env = Env::load()?;
        Ok(Self {
            account: Account {
                id: env.required("CLOUDFLARE_ACCOUNT_ID")?,
            },
            token: env.required("CLOUDFLARE_API_TOKEN")?,
            zone: Zone {
                name: env
                    .get("CLOUDFLARE_ZONE_NAME")
                    .unwrap_or_else(|| "perish.uk".to_string()),
            },
            manage: Manage {
                host: env
                    .get("CLOUDFLARE_MANAGE_HOST")
                    .unwrap_or_else(|| "runseal.perish.uk".to_string()),
                origin: env
                    .get("CLOUDFLARE_MANAGE_ORIGIN_HOST")
                    .unwrap_or_else(|| "releases.runseal.perish.uk".to_string()),
                prefix: env
                    .get("CLOUDFLARE_MANAGE_REDIRECT_PREFIX")
                    .map(|value| value.trim_matches('/').to_string())
                    .unwrap_or_default(),
            },
        })
    }
}

impl Env {
    fn load() -> Result<Self> {
        let path = Self::path();
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("missing secrets file: {}", path.display()))?;
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
        Ok(Self(values))
    }

    fn path() -> PathBuf {
        let secrets = std::env::var_os("RUNSEAL_REPO_SECRETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".local").join("secrets"));
        secrets.join("cloudflare.env")
    }

    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).filter(|value| !value.is_empty()).cloned()
    }

    fn required(&self, key: &str) -> Result<String> {
        self.get(key).with_context(|| {
            format!(
                "missing required key(s) in {}: {key}",
                Self::path().display()
            )
        })
    }
}

pub(super) fn request(
    config: &Config,
    method: &str,
    path: &str,
    query: Vec<(String, String)>,
    body: Option<JsonValue>,
) -> Result<Option<String>> {
    let base = std::env::var("RUNSEAL_CLOUDFLARE_API_BASE")
        .unwrap_or_else(|_| "https://api.cloudflare.com/client/v4".to_string());
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
        .bearer_auth(&config.token)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json");
    if !query.is_empty() {
        request = request.query(&query);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .with_context(|| format!("Cloudflare API {method} {path} unreachable"))?;
    let status = response.status();
    let raw = response
        .text()
        .with_context(|| format!("Cloudflare API {method} {path} returned unreadable body"))?;
    if !status.is_success() {
        bail!(
            "Cloudflare API {method} {path} -> {}: {raw}",
            status.as_u16()
        );
    }
    let payload: JsonValue = if raw.trim().is_empty() {
        JsonValue::Object(Default::default())
    } else {
        serde_json::from_str(&raw)
            .with_context(|| format!("Cloudflare API returned invalid JSON for {path}"))?
    };
    if payload
        .get("success")
        .and_then(JsonValue::as_bool)
        .is_some_and(|success| !success)
    {
        bail!("Cloudflare API {method} {path} failed: {payload}");
    }
    Ok(Some(serde_json::to_string(&payload)?))
}

pub(super) fn required(args: &[String], name: &str) -> Result<String> {
    optional(args, name).ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

pub(super) fn optional(args: &[String], name: &str) -> Option<String> {
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
