use anyhow::{Context, Result, bail};
use serde_json::Value;

pub fn send(verb: &str, url: &str, auth: Option<&str>, body: Option<&Value>) -> Result<Value> {
    let (code, value) = raw(verb, url, auth, body)?;
    if (200..300).contains(&code) {
        return Ok(value);
    }
    let detail = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("request failed");
    bail!("{detail}")
}

pub fn raw(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<(u16, Value)> {
    let done = call(verb, url, auth, body)?;
    let code = done.status().as_u16();
    let text = done
        .into_body()
        .read_to_string()
        .context("request response is unreadable")?;
    if text.trim().is_empty() {
        if (200..300).contains(&code) {
            return Ok((code, Value::Null));
        }
        bail!("request failed");
    }
    let value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) if (200..300).contains(&code) => Value::String(text),
        Err(_) => bail!("response is not JSON"),
    };
    Ok((code, value))
}

fn call(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<ureq::http::Response<ureq::Body>> {
    let held = agent();
    match verb {
        "GET" => done(mark(held.get(url), auth).call()),
        "DELETE" => done(mark(held.delete(url), auth).call()),
        "POST" => write(held.post(url), auth, body),
        "PATCH" => write(held.patch(url), auth, body),
        "PUT" => write(held.put(url), auth, body),
        other => bail!("unsupported HTTP method {other}"),
    }
}

fn mark<B>(request: ureq::RequestBuilder<B>, auth: Option<&str>) -> ureq::RequestBuilder<B> {
    match auth {
        Some(auth) => request.header("Authorization", auth),
        None => request,
    }
}

fn write(
    request: ureq::RequestBuilder<ureq::typestate::WithBody>,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<ureq::http::Response<ureq::Body>> {
    let text = match body {
        Some(body) => serde_json::to_string(body).context("unable to encode JSON")?,
        None => String::new(),
    };
    done(
        mark(request, auth)
            .header("Content-Type", "application/json")
            .send(text),
    )
}

fn done(
    held: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<ureq::http::Response<ureq::Body>> {
    held.context("request failed")
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build(),
    )
}
