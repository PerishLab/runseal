use super::{Class, Error, fault::response};
use crate::http::wire;
use serde_json::Value;
use std::time::Duration;
use zeroize::Zeroize;

pub(super) struct Body(pub Value);

impl Drop for Body {
    fn drop(&mut self) {
        erase(&mut self.0);
    }
}

fn erase(value: &mut Value) {
    match value {
        Value::String(held) => held.zeroize(),
        Value::Array(held) => held.iter_mut().for_each(erase),
        Value::Object(held) => held.values_mut().for_each(erase),
        _ => (),
    }
}

pub(super) struct Record {
    pub body: Body,
    pub request: Option<String>,
}

pub(super) fn agent() -> wire::Client {
    wire::Client::new(wire::Policy {
        redirects: 0,
        overall: Some(Duration::from_secs(20)),
        connect: Some(Duration::from_secs(5)),
        limit: 2 * 1024 * 1024,
    })
}

pub(super) fn invoke(agent: &wire::Client, request: wire::Request<'_>) -> Result<Record, Error> {
    let payload = agent
        .send(request)
        .map_err(|_| Error::unknown(Class::Network))?
        .collect()
        .map_err(|_| response())?;
    decode(payload)
}

pub(super) fn decode(payload: wire::Payload) -> Result<Record, Error> {
    let request = payload
        .headers
        .get("x-tt-logid")
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .filter(|id| id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        .map(|id| id.to_string());
    let body = serde_json::from_slice(&payload.body).map(Body);
    let mut error = match body {
        Ok(body) if payload.status == 200 && body.0["code"].as_i64() == Some(0) => {
            return Ok(Record { body, request });
        }
        Ok(body) if payload.status < 500 => Error::provider(body.0["code"].as_i64()),
        Ok(_) => response(),
        Err(_) => response(),
    };
    error.status = Some(payload.status);
    error.request = request;
    Err(error)
}

#[cfg(test)]
#[path = "tests/wire.rs"]
mod tests;
