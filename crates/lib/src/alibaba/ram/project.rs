use super::{policy, provider, role};
use crate::alibaba::Outcome;
use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "lowercase")]
pub enum Slot<T> {
    Missing,
    Null,
    Value(T),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Provider { name: String },
    Role { name: String },
    Policy { name: String },
    Version { name: String, version: String },
    Attachment { role: String, policy: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
#[non_exhaustive]
pub enum Data {
    Provider(provider::Data),
    Role(role::Data),
    Policy(policy::Data),
    Version(policy::Version),
    Policies(Vec<policy::Entry>),
    Ack,
}

#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub account: String,
    pub principal: String,
    pub action: String,
    pub requested: Target,
    pub request: String,
    pub outcome: Outcome,
    pub data: Data,
    pub complete: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Shape {
    Text,
    Number,
    Boolean,
    Object,
    Array,
}

impl Shape {
    fn accepts(self, value: &Value) -> bool {
        if value.is_null() {
            return true;
        }
        match self {
            Self::Text => value.is_string(),
            Self::Number => value.as_u64().is_some(),
            Self::Boolean => value.is_boolean(),
            Self::Object => value.is_object(),
            Self::Array => value.is_array(),
        }
    }
}

pub(super) fn complete(value: &Value, fields: &[(&str, Shape)]) -> bool {
    value.as_object().is_some_and(|object| {
        object.iter().all(|(name, value)| {
            fields
                .iter()
                .any(|(known, shape)| name == known && shape.accepts(value))
        })
    })
}

pub(super) fn slot<T: DeserializeOwned>(value: &Value, name: &str) -> Result<Slot<T>> {
    match value.get(name) {
        None => Ok(Slot::Missing),
        Some(Value::Null) => Ok(Slot::Null),
        Some(value) => serde_json::from_value(value.clone())
            .map(Slot::Value)
            .map_err(|_| anyhow::anyhow!("Alibaba RAM response field shape refused")),
    }
}

pub(super) use crate::alibaba::model::field as text;

pub(super) fn object<'a>(value: &'a Value, name: &str) -> Result<&'a Value> {
    let row = value
        .get(name)
        .filter(|v| v.is_object())
        .ok_or_else(|| anyhow::anyhow!("Alibaba RAM response object absent or malformed"))?;
    Ok(row)
}

pub(super) fn arn(value: &str, account: &str, kind: &str, name: &str) -> Result<()> {
    if value != format!("acs:ram::{account}:{kind}/{name}") {
        bail!("Alibaba RAM resource ARN differs from the exact owning target");
    }
    Ok(())
}
