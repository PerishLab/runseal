use super::super::api::{Collection, Number, text};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(super) struct User {
    #[serde(rename = "Uin")]
    uin: Number,
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "Uid", skip_serializing_if = "Option::is_none")]
    uid: Option<Number>,
    #[serde(rename = "Remark", skip_serializing_if = "Option::is_none")]
    remark: Option<String>,
    #[serde(rename = "ConsoleLogin", skip_serializing_if = "Option::is_none")]
    console: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Policies {
    #[serde(rename = "List")]
    list: Collection<Policy>,
    #[serde(rename = "TotalNum", skip_serializing_if = "Option::is_none")]
    total: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Policy {
    #[serde(rename = "PolicyId")]
    id: u64,
    #[serde(rename = "PolicyName", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "Type", skip_serializing_if = "Option::is_none")]
    kind: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Document {
    #[serde(rename = "PolicyDocument", deserialize_with = "text")]
    document: String,
    #[serde(rename = "PolicyName", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "Type", skip_serializing_if = "Option::is_none")]
    kind: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Created {
    #[serde(rename = "PolicyId")]
    id: u64,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Keys {
    #[serde(rename = "AccessKeys")]
    keys: Collection<Key>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Issued {
    #[serde(rename = "AccessKey")]
    key: Key,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Key {
    #[serde(rename = "AccessKeyId", deserialize_with = "text")]
    id: String,
    #[serde(rename = "Status", skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(rename = "CreateTime", skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Ack {}
