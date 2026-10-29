mod fault;
mod record;
pub(super) mod sign;
pub(super) use record::metadata;

pub use fault::{Class, Error, Failure, Outcome};

use anyhow::{Result, bail};
use serde_json::Value;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

use super::model::{Credential, now};
use crate::http::wire;

pub(super) struct Body(pub Value);

pub(super) struct Request<'a> {
    pub service: &'a str,
    pub action: &'a str,
    pub region: Option<&'a str>,
    pub params: &'a Value,
}

pub(super) struct Operation<'a> {
    action: &'a str,
    version: &'a str,
    region: Option<&'a str>,
}

impl<'a> Operation<'a> {
    pub fn new(action: &'a str, version: &'a str, region: Option<&'a str>) -> Self {
        Self {
            action,
            version,
            region,
        }
    }
}

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
        overall: Some(Duration::from_secs(15)),
        connect: Some(Duration::from_secs(5)),
        limit: 2 * 1024 * 1024,
    })
}

pub(super) fn post(agent: &wire::Client, url: &str, body: &Value) -> Result<Body> {
    let encoded = Zeroizing::new(
        serde_json::to_string(body).map_err(|_| anyhow::anyhow!("Tencent request refused"))?,
    );
    let response = agent.send(wire::Request {
        method: "POST", url,
        headers: &[("Content-Type", "application/json")],
        body: Some(encoded.as_bytes()),
    }).map_err(|_| anyhow::anyhow!("Tencent transport result is uncertain; do not replay a mutation; inspect the exact resource before recovery"))?;
    decode(response)
}

fn decode(response: wire::Response) -> Result<Body> {
    if response.status != 200 {
        return Err(Failure::status(response.status).into());
    }
    let payload = response.collect().map_err(|_| {
        anyhow::anyhow!("Tencent response unreadable; a mutation may have completed")
    })?;
    let value = serde_json::from_slice(&payload.body).map_err(|_| {
        anyhow::anyhow!("Tencent response malformed; a mutation may have completed")
    })?;
    Ok(Body(value))
}

pub(super) struct Client<'a> {
    credential: &'a Credential,
    agent: wire::Client,
}

impl<'a> Client<'a> {
    pub fn new(credential: &'a Credential) -> Self {
        Self::using(credential, agent())
    }

    pub(super) fn using(credential: &'a Credential, agent: wire::Client) -> Self {
        Self { credential, agent }
    }

    pub fn send(&self, service: &str, action: &str, params: &Value) -> Result<Body> {
        self.invoke(Request {
            service,
            action,
            region: Some("ap-guangzhou"),
            params,
        })
    }

    pub fn invoke(&self, request: Request<'_>) -> Result<Body> {
        let version = version(request.service)?;
        let host = format!("{}.tencentcloudapi.com", request.service);
        self.request(
            (&format!("https://{host}"), &host),
            Operation::new(request.action, version, request.region),
            request.params,
        )
    }

    pub(super) fn request(
        &self,
        origin: (&str, &str),
        operation: Operation<'_>,
        params: &Value,
    ) -> Result<Body> {
        let timestamp = now();
        let body = Zeroizing::new(
            serde_json::to_string(params)
                .map_err(|_| anyhow::anyhow!("Tencent request refused"))?,
        );
        let service = origin
            .1
            .split('.')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Tencent service refused"))?;
        let authorization = Zeroizing::new(sign::authorization(
            self.credential,
            (origin.1, service, operation.action),
            &body,
            timestamp,
        )?);
        let timestamp = timestamp.to_string();
        let mut headers = vec![
            ("Content-Type", "application/json; charset=utf-8"),
            ("Authorization", authorization.as_str()),
            ("Host", origin.1),
            ("X-TC-Action", operation.action),
            ("X-TC-Version", operation.version),
            ("X-TC-Timestamp", timestamp.as_str()),
        ];
        if let Some(region) = operation.region {
            headers.push(("X-TC-Region", region));
        }
        if let Some(token) = &self.credential.token {
            headers.push(("X-TC-Token", token.as_str()));
        }
        let response = self.agent.send(wire::Request {
            method: "POST", url: origin.0,
            headers: &headers, body: Some(body.as_bytes()),
        }).map_err(|_| Failure::new(Class::Network, "Tencent transport result is uncertain; do not replay a mutation; inspect the exact resource before recovery"))?;
        let mut decoded = decode(response)?;
        let value = decoded
            .0
            .get_mut("Response")
            .ok_or_else(|| anyhow::anyhow!("Tencent response refused"))?;
        if value.get("Error").is_some() {
            return Err(Failure::provider(value).into());
        }
        if value
            .get("RequestId")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            bail!("Tencent success receipt missing; a mutation may have completed");
        }
        Ok(Body(value.take()))
    }
}

pub(super) fn version(service: &str) -> Result<&'static str> {
    match service {
        "cam" => Ok("2019-01-16"),
        "dnspod" => Ok("2021-03-23"),
        "sts" => Ok("2018-08-13"),
        "vpc" => Ok("2017-03-12"),
        "cvm" => Ok("2017-03-12"),
        "lighthouse" => Ok("2020-03-24"),
        "region" => Ok("2022-06-27"),
        _ => bail!("Tencent service refused"),
    }
}

#[cfg(test)]
mod tests;
