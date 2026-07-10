use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

use super::cloudflare::{load_config, optional_option, request, required_option};

pub(super) fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "list" => list(args),
        "create" => create(args),
        "update" => update(args),
        _ => bail!("usage: runseal @tool cloudflare zone dns-record list|create|update ..."),
    }
}

fn list(args: &[String]) -> Result<Option<String>> {
    let zone_id = required_option(args, "--zone-id")?;
    let query = optional_option(args, "--name")
        .map(|name| vec![("name".to_string(), name)])
        .unwrap_or_default();
    let config = load_config()?;
    let payload = request(
        &config,
        "GET",
        &format!("/zones/{zone_id}/dns_records"),
        query,
        None,
    )?;
    let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
    Ok(Some(serde_json::to_string(
        value.get("result").unwrap_or(&JsonValue::Array(Vec::new())),
    )?))
}

fn create(args: &[String]) -> Result<Option<String>> {
    let zone_id = required_option(args, "--zone-id")?;
    let body = record_json(args)?;
    let config = load_config()?;
    let payload = request(
        &config,
        "POST",
        &format!("/zones/{zone_id}/dns_records"),
        Vec::new(),
        Some(body),
    )?;
    result(payload)
}

fn update(args: &[String]) -> Result<Option<String>> {
    let zone_id = required_option(args, "--zone-id")?;
    let record_id = required_option(args, "--record-id")?;
    let body = record_json(args)?;
    let config = load_config()?;
    let payload = request(
        &config,
        "PATCH",
        &format!("/zones/{zone_id}/dns_records/{record_id}"),
        Vec::new(),
        Some(body),
    )?;
    result(payload)
}

fn record_json(args: &[String]) -> Result<JsonValue> {
    let record_json = required_option(args, "--json")?;
    serde_json::from_str(&record_json).context("invalid DNS record JSON")
}

fn result(payload: Option<String>) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
    Ok(Some(serde_json::to_string(
        value.get("result").unwrap_or(&JsonValue::Null),
    )?))
}
