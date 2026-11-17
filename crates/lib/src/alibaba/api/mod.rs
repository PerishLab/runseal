pub(super) mod sign;

use anyhow::{Result, bail};
use serde_json::Value;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

use super::fault::{Class, Failure};
use super::model::Credential;
use crate::http::wire;

pub(super) const STS: &str = "https://sts.aliyuncs.com";
pub(super) const HOST: &str = "sts.aliyuncs.com";
pub(super) const VERSION: &str = "2015-04-01";

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

pub(super) fn agent() -> wire::Client {
    wire::Client::new(wire::Policy {
        redirects: 0,
        overall: Some(Duration::from_secs(20)),
        connect: Some(Duration::from_secs(5)),
        limit: super::model::BODY as u64,
    })
}

pub(super) fn receive(
    agent: &wire::Client,
    request: wire::Request<'_>,
) -> std::result::Result<(u16, Zeroizing<Vec<u8>>), wire::Fault> {
    let response = agent.send(request)?;
    let status = response.status;
    let payload = response.collect()?;
    Ok((status, payload.body))
}

pub(super) fn decode(bytes: &[u8], class: Class) -> Result<Body> {
    let value = serde_json::from_slice(bytes).map_err(|_| {
        Failure::new(
            class,
            "Alibaba response body was not the expected JSON document; a token operation may have completed; inspect status before recovery",
        )
    })?;
    Ok(Body(value))
}

pub(super) fn identity(credential: &Credential, agent: &wire::Client) -> Result<Body> {
    let query = "";
    let date = sign::date(super::model::now())?;
    let nonce = nonce()?;
    let digest = sign::hash(b"");
    let payload = [
        ("host", HOST),
        ("x-acs-action", "GetCallerIdentity"),
        ("x-acs-content-sha256", digest.as_str()),
        ("x-acs-date", date.as_str()),
        ("x-acs-signature-nonce", nonce.as_str()),
        ("x-acs-version", VERSION),
        ("x-acs-security-token", credential.token.as_str()),
    ];
    let mut sorted = payload.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    let authorization = Zeroizing::new(sign::authorization(
        (credential.id.as_str(), credential.key.as_str()),
        &sorted,
        query,
    )?);
    let mut headers = sorted;
    headers.push(("Authorization", authorization.as_str()));
    let url = format!("{STS}/");
    let (status, bytes) = match receive(
        agent,
        wire::Request {
            method: "POST",
            url: &url,
            headers: &headers,
            body: None,
        },
    ) {
        Ok(value) => value,
        Err(_) => {
            return Err(Failure::network(
                "Alibaba identity transport result was not observed; no automatic retry",
            )
            .into());
        }
    };
    let body = decode(&bytes, Class::Response)?;
    let request = body
        .0
        .get("RequestId")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if status >= 500 {
        return Err(Failure::status(status).request(request.as_deref()).into());
    }
    if status != 200 {
        let code = body
            .0
            .get("Code")
            .and_then(Value::as_str)
            .map(str::to_owned);
        return Err(Failure::acs(status, code.as_deref(), request.as_deref()).into());
    }
    if request.is_none_or(|id| id.is_empty()) {
        bail!("Alibaba identity receipt missing; the caller was not established");
    }
    Ok(body)
}

fn nonce() -> Result<String> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random)
        .map_err(|_| anyhow::anyhow!("Alibaba request entropy unavailable"))?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}
