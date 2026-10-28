use anyhow::{Result, bail};
use serde_json::Value;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

use super::{
    model::{Credential, now},
    sign,
};

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
        bail!(
            "Tencent HTTP response refused ({}); the remote operation may have completed; no automatic retry",
            response.status().as_u16()
        );
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
        let version = match service {
            "cam" => "2019-01-16",
            "dnspod" => "2021-03-23",
            "sts" => "2018-08-13",
            _ => bail!("Tencent service refused"),
        };
        let host = format!("{service}.tencentcloudapi.com");
        self.request(
            (&format!("https://{host}"), &host),
            (action, version),
            params,
        )
    }

    pub(super) fn request(
        &self,
        origin: (&str, &str),
        operation: (&str, &str),
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
            (origin.1, service, operation.0),
            &body,
            timestamp,
        )?);
        let mut request = self
            .agent
            .post(origin.0)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Authorization", authorization.as_str())
            .header("Host", origin.1)
            .header("X-TC-Action", operation.0)
            .header("X-TC-Version", operation.1)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Region", "ap-guangzhou");
        if let Some(token) = &self.credential.token {
            request = request.header("X-TC-Token", token);
        }
        let response = request.send(body.as_bytes()).map_err(|_| anyhow::anyhow!("Tencent transport result is uncertain; do not replay a mutation; inspect the exact resource before recovery"))?;
        let mut decoded = decode(response)?;
        let value = decoded
            .0
            .get_mut("Response")
            .ok_or_else(|| anyhow::anyhow!("Tencent response refused"))?;
        if let Some(error) = value.get("Error") {
            let reason = match error
                .get("Code")
                .and_then(Value::as_str)
                .unwrap_or_default()
            {
                "UnauthorizedOperation" | "AuthFailure.UnauthorizedOperation" => {
                    "required permission was refused"
                }
                "AuthFailure.SignatureFailure" => "signature was refused",
                "AuthFailure.TokenFailure"
                | "AuthFailure.SecretIdNotFound"
                | "AuthFailure.InvalidSecretId" => "credentials were refused",
                "RequestLimitExceeded" => "request limit was reached",
                _ => "operation was refused",
            };
            bail!("Tencent {reason}; no provider message or raw response is emitted");
        }
        if value
            .get("RequestId")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            bail!("Tencent success receipt missing; a mutation may have completed");
        }
        receipt(operation.0, value)?;
        Ok(Body(value.take()))
    }
}

fn receipt(action: &str, value: &Value) -> Result<()> {
    let valid = match action {
        "GetCallerIdentity" => value.get("AccountId").is_some_and(Value::is_string),
        "GetUser" | "AddUser" => value
            .get("Uin")
            .is_some_and(|held| held.is_u64() || held.is_string()),
        "ListPolicies" => value
            .get("List")
            .is_some_and(|held| held.is_array() || held.is_null()),
        "GetPolicy" => value.get("PolicyDocument").is_some_and(Value::is_string),
        "CreatePolicy" => value.get("PolicyId").is_some_and(Value::is_u64),
        "ListAccessKeys" => value
            .get("AccessKeys")
            .is_some_and(|held| held.is_array() || held.is_null()),
        "CreateAccessKey" => value.get("AccessKey").is_some_and(Value::is_object),
        "DescribeDomain" => value.get("DomainInfo").is_some_and(Value::is_object),
        "DescribeRecordList" => value.get("RecordList").is_some_and(Value::is_array),
        "CreateRecord" | "ModifyRecord" => value.get("RecordId").is_some_and(Value::is_u64),
        "AttachUserPolicy" | "DetachUserPolicy" | "UpdateAccessKey" | "DeleteAccessKey"
        | "DeleteRecord" => true,
        _ => false,
    };
    if !valid {
        bail!(
            "Tencent operation receipt incomplete; a mutation may have completed; inspect the exact resource before recovery"
        );
    }
    Ok(())
}

const FIELDS: &[&str] = &[
    "RequestId",
    "AccountId",
    "UserId",
    "PrincipalId",
    "Type",
    "Arn",
    "Uin",
    "Uid",
    "Name",
    "Remark",
    "ConsoleLogin",
    "UseApi",
    "PolicyId",
    "PolicyName",
    "Description",
    "PolicyDocument",
    "PolicyType",
    "List",
    "TotalNum",
    "AccessKeyId",
    "AccessKeys",
    "Status",
    "CreateTime",
    "RecordId",
    "RecordList",
    "RecordCountInfo",
    "TotalCount",
    "SubdomainCount",
    "ListCount",
    "DomainInfo",
    "Id",
    "Domain",
    "TTL",
    "Line",
    "LineId",
    "Value",
    "MX",
    "UpdatedOn",
    "MonitorStatus",
    "DefaultNS",
    "IsMark",
    "IsPaused",
    "Grade",
    "GradeLevel",
    "GradeTitle",
    "IsVip",
    "Owner",
    "Records",
    "IsPendingShare",
    "Punycode",
    "RouteId",
    "DomainId",
    "ValueCount",
];

pub(super) fn metadata(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(name, _)| FIELDS.contains(&name.as_str()))
                .map(|(name, held)| (name.clone(), metadata(held)))
                .collect::<serde_json::Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.iter().take(200).map(metadata).collect()),
        Value::String(text) if text.len() > 128 * 1024 => Value::Null,
        other => other.clone(),
    }
}
