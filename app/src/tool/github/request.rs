use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

const BASE: &str = "https://api.github.com";
const VERSION: &str = "2022-11-28";

pub(super) fn text(method: &str, path: &str, token: &str, body: String) -> Result<Option<String>> {
    send(
        method,
        path,
        Some(token),
        Some(serde_json::json!({
            "body": body,
        })),
    )
    .map(|payload| Some(serde_json::to_string(&payload).expect("GitHub payload should serialize")))
}

pub(super) fn send(
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<JsonValue>,
) -> Result<JsonValue> {
    let base = std::env::var("RUNSEAL_GITHUB_API_BASE").unwrap_or_else(|_| BASE.to_string());
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
        .header("X-GitHub-Api-Version", VERSION)
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
