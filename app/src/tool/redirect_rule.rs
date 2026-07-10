use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

use super::cloudflare::{optional_option, required_option};

pub(super) fn eval(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool cloudflare redirect-rule exact ...");
    };
    if command != "exact" {
        bail!("usage: runseal @tool cloudflare redirect-rule exact ...");
    }
    let ref_name = required_option(rest, "--ref")?;
    let description = required_option(rest, "--description")?;
    let host = required_option(rest, "--host")?;
    let path = required_option(rest, "--path")?;
    let target_url = required_option(rest, "--target-url")?;
    let status_code = optional_option(rest, "--status-code")
        .unwrap_or_else(|| "302".to_string())
        .parse::<u16>()
        .context("invalid redirect status code")?;
    Ok(Some(serde_json::to_string(&redirect(
        ref_name,
        description,
        host,
        path,
        target_url,
        status_code,
    ))?))
}

fn redirect(
    ref_name: String,
    description: String,
    host: String,
    path: String,
    target_url: String,
    status_code: u16,
) -> JsonValue {
    serde_json::json!({
        "ref": ref_name,
        "description": description,
        "expression": format!("(http.host eq \"{host}\" and http.request.uri.path eq \"{path}\")"),
        "action": "redirect",
        "enabled": true,
        "action_parameters": params(target_url, status_code),
    })
}

fn params(target_url: String, status_code: u16) -> JsonValue {
    serde_json::json!({
        "from_value": from(target_url, status_code),
    })
}

fn from(target_url: String, status_code: u16) -> JsonValue {
    serde_json::json!({
        "target_url": target(target_url),
        "status_code": status_code,
        "preserve_query_string": false,
    })
}

fn target(value: String) -> JsonValue {
    serde_json::json!({
        "value": value,
    })
}
