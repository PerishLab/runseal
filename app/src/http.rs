use std::{collections::BTreeMap, error::Error, fmt};

use anyhow::{Context, Result, bail};
use serde_json::Value;

pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

pub struct Fault {
    response: Response,
}

pub struct Request<'a> {
    verb: &'a str,
    url: &'a str,
    headers: BTreeMap<String, String>,
    body: Option<&'a Value>,
}

impl<'a> Request<'a> {
    pub fn new(verb: &'a str, url: &'a str) -> Self {
        Self {
            verb,
            url,
            headers: BTreeMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn json(mut self, body: &'a Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn send(self) -> Result<Response> {
        exchange(&self)
    }
}

impl Fault {
    pub fn response(&self) -> &Response {
        &self.response
    }

    pub fn status(&self) -> u16 {
        self.response.status
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.response
            .headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn body(&self) -> &Value {
        &self.response.body
    }
}

impl fmt::Debug for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fault")
            .field("status", &self.response.status)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.response.body.get("message").and_then(Value::as_str) {
            Some(message) => f.write_str(message),
            None => write!(
                f,
                "HTTP request failed with status {}",
                self.response.status
            ),
        }
    }
}

impl Error for Fault {}

pub fn send(verb: &str, url: &str, auth: Option<&str>, body: Option<&Value>) -> Result<Value> {
    let response = request(verb, url, auth, body)?;
    if (200..300).contains(&response.status) {
        return Ok(response.body);
    }
    Err(Fault { response }.into())
}

pub fn raw(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<(u16, Value)> {
    let response = request(verb, url, auth, body)?;
    Ok((response.status, response.body))
}

pub fn request(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<Response> {
    let mut request = Request::new(verb, url);
    if let Some(auth) = auth {
        request = request.header("Authorization", auth);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    request.send()
}

fn exchange(request: &Request<'_>) -> Result<Response> {
    let done = call(request)?;
    let code = done.status().as_u16();
    let headers = done
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect();
    let text = done
        .into_body()
        .read_to_string()
        .context("request response is unreadable")?;
    if text.trim().is_empty() {
        return Ok(Response {
            status: code,
            headers,
            body: Value::Null,
        });
    }
    let value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => Value::String(text),
    };
    Ok(Response {
        status: code,
        headers,
        body: value,
    })
}

fn call(request: &Request<'_>) -> Result<ureq::http::Response<ureq::Body>> {
    let held = agent();
    match request.verb {
        "GET" => done(mark(held.get(request.url), request).call()),
        "DELETE" => done(mark(held.delete(request.url), request).call()),
        "POST" => write(held.post(request.url), request),
        "PATCH" => write(held.patch(request.url), request),
        "PUT" => write(held.put(request.url), request),
        other => bail!("unsupported HTTP method {other}"),
    }
}

fn mark<B>(mut held: ureq::RequestBuilder<B>, request: &Request<'_>) -> ureq::RequestBuilder<B> {
    for (name, value) in &request.headers {
        held = held.header(name, value);
    }
    held
}

fn write(
    held: ureq::RequestBuilder<ureq::typestate::WithBody>,
    request: &Request<'_>,
) -> Result<ureq::http::Response<ureq::Body>> {
    let text = match request.body {
        Some(body) => serde_json::to_string(body).context("unable to encode JSON")?,
        None => String::new(),
    };
    done(
        mark(held, request)
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
