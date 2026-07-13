use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

use super::{Config, optional, request, required};

pub(super) fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "list" => list(args),
        "create" => create(args),
        "update" => update(args),
        _ => bail!("usage: runseal @tool cloudflare zone dns-record list|create|update ..."),
    }
}

fn list(args: &[String]) -> Result<Option<String>> {
    let zone = required(args, "--zone-id")?;
    let query = optional(args, "--name")
        .map(|name| vec![("name".to_string(), name)])
        .unwrap_or_default();
    let config = Config::load()?;
    let payload = request(
        &config,
        "GET",
        &format!("/zones/{zone}/dns_records"),
        query,
        None,
    )?;
    let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
    Ok(Some(serde_json::to_string(
        value.get("result").unwrap_or(&JsonValue::Array(Vec::new())),
    )?))
}

fn create(args: &[String]) -> Result<Option<String>> {
    let zone = required(args, "--zone-id")?;
    let body = payload(args)?;
    let config = Config::load()?;
    let payload = request(
        &config,
        "POST",
        &format!("/zones/{zone}/dns_records"),
        Vec::new(),
        Some(body),
    )?;
    result(payload)
}

fn update(args: &[String]) -> Result<Option<String>> {
    let zone = required(args, "--zone-id")?;
    let record = required(args, "--record-id")?;
    let body = payload(args)?;
    let config = Config::load()?;
    let payload = request(
        &config,
        "PATCH",
        &format!("/zones/{zone}/dns_records/{record}"),
        Vec::new(),
        Some(body),
    )?;
    result(payload)
}

fn payload(args: &[String]) -> Result<JsonValue> {
    let raw = required(args, "--json")?;
    serde_json::from_str(&raw).context("invalid DNS record JSON")
}

fn result(payload: Option<String>) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
    Ok(Some(serde_json::to_string(
        value.get("result").unwrap_or(&JsonValue::Null),
    )?))
}
