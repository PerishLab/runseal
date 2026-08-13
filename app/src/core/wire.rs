use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn get(url: &str, token: &str) -> Result<Value> {
    let done = agent()
        .get(url)
        .header("Authorization", &format!("token {token}"))
        .call()
        .context("forgejo request failed")?;
    let code = done.status();
    let text = done
        .into_body()
        .read_to_string()
        .context("forgejo response is unreadable")?;
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

pub fn envelope(value: &Value) -> Result<String> {
    let body = match value {
        Value::Array(_) => json!({"version": 1, "issues": value}),
        object if object.get("login").is_some() => json!({"version": 1, "user": value}),
        _ => json!({"version": 1, "issue": value}),
    };
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
    let number = value
        .get("number")
        .map(|held| held.to_string())
        .unwrap_or_default();
    let state = value.get("state").and_then(Value::as_str).unwrap_or("");
    let title = value.get("title").and_then(Value::as_str).unwrap_or("");
    format!("{number}\t{state}\t{title}")
}
