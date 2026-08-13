use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn get(url: &str, token: &str) -> Result<Value> {
    finish(
        agent()
            .get(url)
            .header("Authorization", &format!("token {token}"))
            .call(),
    )
}

pub fn post(url: &str, token: &str, body: &Value) -> Result<Value> {
    let text = serde_json::to_string(body).context("unable to encode JSON")?;
    finish(
        agent()
            .post(url)
            .header("Authorization", &format!("token {token}"))
            .header("Content-Type", "application/json")
            .send(text),
    )
}

pub fn patch(url: &str, token: &str, body: &Value) -> Result<Value> {
    let text = serde_json::to_string(body).context("unable to encode JSON")?;
    finish(
        agent()
            .patch(url)
            .header("Authorization", &format!("token {token}"))
            .header("Content-Type", "application/json")
            .send(text),
    )
}

fn finish(
    done: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Value> {
    let done = done.context("forgejo request failed")?;
    let code = done.status();
    let text = done
        .into_body()
        .read_to_string()
        .context("forgejo response is unreadable")?;
    if text.trim().is_empty() {
        if code.is_success() {
            return Ok(Value::Null);
        }
        bail!("forgejo: request failed");
    }
    let value: Value = serde_json::from_str(&text).context("forgejo response is not JSON")?;
    if code.is_success() {
        return Ok(value);
    }
    reject(value)
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build(),
    )
}

fn reject(value: Value) -> Result<Value> {
    let detail = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("request failed");
    bail!("forgejo: {detail}")
}

pub fn envelope(kind: &str, value: &Value) -> Result<String> {
    let body = json!({"version": 1, kind: value});
    serde_json::to_string(&body).context("unable to encode JSON")
}

pub fn print(value: &Value) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                println!("{}", line(row));
            }
        }
        other => println!("{}", line(other)),
    }
}

fn line(value: &Value) -> String {
    if let Some(login) = value.get("login").and_then(Value::as_str) {
        return login.to_string();
    }
    if let Some(title) = value.get("title").and_then(Value::as_str) {
        let number = value
            .get("number")
            .map(|held| held.to_string())
            .unwrap_or_default();
        let state = value.get("state").and_then(Value::as_str).unwrap_or("");
        return format!("{number}\t{state}\t{title}");
    }
    let id = value
        .get("id")
        .map(|held| held.to_string())
        .unwrap_or_default();
    let login = value
        .pointer("/user/login")
        .and_then(Value::as_str)
        .unwrap_or("");
    let body = value
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("");
    format!("{id}\t{login}\t{body}")
}
