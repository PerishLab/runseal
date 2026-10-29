mod fault;
mod record;
pub(super) mod sign;
pub(super) use record::metadata;

pub use fault::{Class, Failure};

use anyhow::{Result, bail};
use serde_json::Value;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

use super::model::{Credential, now};

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

pub(super) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(15)))
        .timeout_connect(Some(Duration::from_secs(5)))
        .build()
        .into()
}

pub(super) fn post(agent: &ureq::Agent, url: &str, body: &Value) -> Result<Body> {
    let encoded = Zeroizing::new(
        serde_json::to_string(body).map_err(|_| anyhow::anyhow!("Tencent request refused"))?,
    );
    let response = agent.post(url).header("Content-Type", "application/json").send(encoded.as_bytes()).map_err(|_| anyhow::anyhow!("Tencent transport result is uncertain; do not replay a mutation; inspect the exact resource before recovery"))?;
    decode(response)
}

fn decode(mut response: ureq::http::Response<ureq::Body>) -> Result<Body> {
    if response.status().as_u16() != 200 {
        return Err(Failure::status(response.status().as_u16()).into());
    }
    let bytes = Zeroizing::new(
        response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_to_vec()
            .map_err(|_| {
                anyhow::anyhow!("Tencent response unreadable; a mutation may have completed")
            })?,
    );
    let value = serde_json::from_slice(&bytes).map_err(|_| {
        anyhow::anyhow!("Tencent response malformed; a mutation may have completed")
    })?;
    Ok(Body(value))
}

pub(super) struct Client<'a> {
    credential: &'a Credential,
    agent: ureq::Agent,
}

impl<'a> Client<'a> {
    pub fn new(credential: &'a Credential) -> Self {
        Self {
            credential,
            agent: agent(),
        }
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
        let mut request = self
            .agent
            .post(origin.0)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Authorization", authorization.as_str())
            .header("Host", origin.1)
            .header("X-TC-Action", operation.action)
            .header("X-TC-Version", operation.version)
            .header("X-TC-Timestamp", timestamp.to_string());
        if let Some(region) = operation.region {
            request = request.header("X-TC-Region", region);
        }
        if let Some(token) = &self.credential.token {
            request = request.header("X-TC-Token", token);
        }
        let response = request.send(body.as_bytes()).map_err(|_| Failure::new(Class::Network, "Tencent transport result is uncertain; do not replay a mutation; inspect the exact resource before recovery"))?;
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
        record::receipt(operation.action, value)?;
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
