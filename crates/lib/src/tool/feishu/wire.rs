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

pub(super) fn oauth(agent: &wire::Client, request: wire::Request<'_>) -> Result<Record, Error> {
    let payload = agent
        .send(request)
        .map_err(|_| Error::unknown(Class::Network))?
        .collect()
        .map_err(|_| response())?;
    authorization(payload)
}

fn authorization(payload: wire::Payload) -> Result<Record, Error> {
    let request = tracking(&payload);
    if !matches!(payload.status, 200 | 400 | 401 | 403 | 429) {
        let mut error = response();
        error.status = Some(payload.status);
        error.request = request;
        return Err(error);
    }
    let body = Body(serde_json::from_slice(&payload.body).map_err(|_| response())?);
    if payload.status == 200
        && body.0.get("error").is_none()
        && body
            .0
            .get("code")
            .is_none_or(|code| code.as_i64() == Some(0))
    {
        return Ok(Record { body, request });
    }
    let name = body.0["error"].as_str().unwrap_or("");
    let class = match name {
        "authorization_pending" | "slow_down" => Class::Limit,
        "access_denied" | "invalid_scope" | "insufficient_scope" => Class::Permission,
        "expired_token" => Class::Expired,
        "invalid_grant" | "invalid_client" | "unauthorized_client" => Class::Authentication,
        _ => Class::Response,
    };
    let mut error = Error::provider(body.0["code"].as_i64());
    if class != Class::Response {
        error = Error::new(
            class,
            "Feishu OAuth refused or pending; inspect local status and explicit next operation; no automatic retry",
        );
        error.outcome = super::Outcome::Rejected;
        error.poll = match name {
            "authorization_pending" => Some(super::fault::Poll::Pending),
            "slow_down" => Some(super::fault::Poll::Slow),
            _ => None,
        };
    }
    error.status = Some(payload.status);
    error.request = request;
    Err(error)
}

fn tracking(payload: &wire::Payload) -> Option<String> {
    payload
        .headers
        .get("x-tt-logid")
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .filter(|id| id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        .map(|id| id.to_string())
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
