use std::{collections::BTreeMap, error::Error, fmt, thread, time::Duration};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::http;

pub struct Fault {
    detail: Box<Detail>,
}

struct Detail {
    status: u16,
    codes: Vec<i64>,
    messages: Vec<String>,
    ray: Option<String>,
    retry: Option<String>,
    body: Value,
}

impl Fault {
    pub fn status(&self) -> u16 {
        self.detail.status
    }

    pub fn codes(&self) -> &[i64] {
        &self.detail.codes
    }

    pub fn ray(&self) -> Option<&str> {
        self.detail.ray.as_deref()
    }

    pub fn retry(&self) -> Option<&str> {
        self.detail.retry.as_deref()
    }

    pub fn body(&self) -> &Value {
        &self.detail.body
    }
}

impl fmt::Debug for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fault")
            .field("status", &self.detail.status)
            .field("codes", &self.detail.codes)
            .field("ray", &self.detail.ray)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = if self.detail.messages.is_empty() {
            "request failed".to_string()
        } else {
            self.detail.messages.join("; ")
        };
        write!(f, "cloudflare refused ({}): {detail}", self.detail.status)
    }
}

impl Error for Fault {}

pub(super) struct Page {
    pub result: Value,
    pub info: Value,
}

pub(super) struct Client {
    base: String,
    headers: BTreeMap<String, String>,
}

impl Client {
    pub fn new(base: String, headers: BTreeMap<String, String>) -> Self {
        Self { base, headers }
    }

    pub fn send(&self, method: &str, route: &str, body: Option<&Value>) -> Result<Page> {
        let url = format!("{}{route}", self.base);
        let mut turn = 0u32;
        loop {
            let response = self.request(method, &url, body)?.send()?;
            let Some(delay) = delay(method, &response, turn) else {
                return envelope(response).map_err(Into::into);
            };
            thread::sleep(delay);
            turn += 1;
        }
    }

    fn request<'a>(
        &self,
        method: &'a str,
        url: &'a str,
        body: Option<&'a Value>,
    ) -> Result<http::Request<'a>> {
        let mut request = http::Request::new(method, url);
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        Ok(request)
    }
}

fn delay(method: &str, response: &http::Response, turn: u32) -> Option<Duration> {
    let retryable = response.status == 429 || (500..600).contains(&response.status);
    if method != "GET" || !retryable || turn >= 2 {
        return None;
    }
    let seconds = response
        .headers
        .get("retry-after")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1 << turn);
    (seconds <= 30).then(|| Duration::from_secs(seconds))
}

fn envelope(response: http::Response) -> std::result::Result<Page, Fault> {
    let success = response.body.get("success").and_then(Value::as_bool);
    if (200..300).contains(&response.status) && success == Some(true) {
        return Ok(Page {
            result: response.body.get("result").cloned().unwrap_or(Value::Null),
            info: response
                .body
                .get("result_info")
                .cloned()
                .unwrap_or(Value::Null),
        });
    }
    Err(fault(response))
}

fn fault(response: http::Response) -> Fault {
    let entries = response
        .body
        .get("errors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let codes = entries
        .iter()
        .filter_map(|entry| entry.get("code").and_then(Value::as_i64))
        .collect();
    let mut messages = entries
        .iter()
        .filter_map(|entry| entry.get("message").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if messages.is_empty() {
        messages.extend(
            response
                .body
                .get("messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("message").and_then(Value::as_str))
                .map(str::to_string),
        );
    }
    let ray = response
        .headers
        .get("cf-ray")
        .cloned()
        .or_else(|| response.headers.get("cf-ray-id").cloned());
    let retry = response.headers.get("retry-after").cloned();
    Fault {
        detail: Box::new(Detail {
            status: response.status,
            codes,
            messages,
            ray,
            retry,
            body: response.body,
        }),
    }
}

pub(super) fn object(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("unable to read JSON body file {path}"))?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("unable to parse JSON body file {path}"))?;
    if !value.is_object() {
        anyhow::bail!("JSON body file must contain an object: {path}");
    }
    Ok(value)
}
