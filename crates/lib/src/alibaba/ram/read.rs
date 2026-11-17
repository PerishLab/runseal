use super::project::{Shape, complete, object, text};
use super::{Data, Request, policy, provider, role};
use anyhow::{Result, bail};
use serde_json::Value;

pub(super) fn decode(
    value: &Value,
    request: &Request,
    account: &str,
) -> Result<(String, Data, bool)> {
    let receipt = text(value, "RequestId")?;
    if receipt.len() > 128
        || !receipt
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || v == b'-')
    {
        bail!("Alibaba RAM request receipt refused");
    }
    let (data, known) = project(value, request, account)?;
    let root = match request {
        Request::Provider { .. } | Request::Register(_) => complete(
            value,
            &[("RequestId", Shape::Text), ("OIDCProvider", Shape::Object)],
        ),
        Request::Role { .. } | Request::Create(_) => complete(
            value,
            &[("RequestId", Shape::Text), ("Role", Shape::Object)],
        ),
        Request::Policy { .. } | Request::Define(_) => complete(
            value,
            &[
                ("RequestId", Shape::Text),
                ("Policy", Shape::Object),
                ("DefaultPolicyVersion", Shape::Object),
            ],
        ),
        Request::Version { .. } => complete(
            value,
            &[("RequestId", Shape::Text), ("PolicyVersion", Shape::Object)],
        ),
        Request::Policies { .. } => complete(
            value,
            &[("RequestId", Shape::Text), ("Policies", Shape::Object)],
        ),
        Request::Attach { .. } => complete(value, &[("RequestId", Shape::Text)]),
    };
    Ok((receipt.into(), data, known && root))
}

fn project(value: &Value, request: &Request, account: &str) -> Result<(Data, bool)> {
    match request {
        Request::Provider { name } => {
            let (data, known) = provider::decode(object(value, "OIDCProvider")?, name, account)?;
            Ok((Data::Provider(data), known))
        }
        Request::Register(input) => {
            let (data, known) =
                provider::decode(object(value, "OIDCProvider")?, &input.name, account)?;
            if data.issuer != input.issuer {
                bail!("Alibaba created provider issuer differs from the submitted target");
            }
            Ok((Data::Provider(data), known))
        }
        Request::Role { name } => {
            let (data, known) = role::decode(object(value, "Role")?, name, account)?;
            Ok((Data::Role(data), known))
        }
        Request::Create(input) => {
            let (data, known) = role::decode(object(value, "Role")?, &input.name, account)?;
            Ok((Data::Role(data), known))
        }
        Request::Policy { name } => {
            let (data, known) = policy::decode(value, name)?;
            Ok((Data::Policy(data), known))
        }
        Request::Define(input) => {
            let (data, known) = policy::decode(value, &input.name)?;
            Ok((Data::Policy(data), known))
        }
        Request::Version { version, .. } => {
            let (data, known) = policy::version(object(value, "PolicyVersion")?)?;
            if &data.id != version {
                bail!("Alibaba policy version differs from the requested version");
            }
            Ok((Data::Version(data), known))
        }
        Request::Policies { .. } => {
            let (data, known) = policy::entries(value)?;
            Ok((Data::Policies(data), known))
        }
        Request::Attach { .. } => Ok((Data::Ack, true)),
    }
}
