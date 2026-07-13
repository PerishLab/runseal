use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

use super::cmd;

pub(super) fn eval(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool cloudflare redirect-rule exact ...");
    };
    if command != "exact" {
        bail!("usage: runseal @tool cloudflare redirect-rule exact ...");
    }
    let reference = cmd(rest).required("--ref")?;
    let description = cmd(rest).required("--description")?;
    let host = cmd(rest).required("--host")?;
    let path = cmd(rest).required("--path")?;
    let url = cmd(rest).required("--target-url")?;
    let status = cmd(rest)
        .optional("--status-code")
        .unwrap_or_else(|| "302".to_string())
        .parse::<u16>()
        .context("invalid redirect status code")?;
    Ok(Some(serde_json::to_string(&redirect(
        reference,
        description,
        host,
        path,
        url,
        status,
    ))?))
}

fn redirect(
    reference: String,
    description: String,
    host: String,
    path: String,
    url: String,
    status: u16,
) -> JsonValue {
    serde_json::json!({
        "ref": reference,
        "description": description,
        "expression": format!("(http.host eq \"{host}\" and http.request.uri.path eq \"{path}\")"),
        "action": "redirect",
        "enabled": true,
        "action_parameters": params(url, status),
    })
}

fn params(url: String, status: u16) -> JsonValue {
    serde_json::json!({
        "from_value": from(url, status),
    })
}

fn from(url: String, status: u16) -> JsonValue {
    serde_json::json!({
        "target_url": target(url),
        "status_code": status,
        "preserve_query_string": false,
    })
}

fn target(value: String) -> JsonValue {
    serde_json::json!({
        "value": value,
    })
}
