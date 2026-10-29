use super::{Import, Instance, Key, Price, Quota, Status};
use crate::tencent::{Identity, Outcome};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Data {
    Quote(Box<Price>),
    Quota(Vec<Quota>),
    Created(Vec<String>),
    Imported(String),
    Instance(Option<Box<Instance>>),
    Status(Option<Status>),
    Key(Option<Key>),
    Checked,
    Acknowledged(String),
}

#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub identity: Identity,
    pub service: String,
    pub version: String,
    pub action: String,
    pub region: String,
    pub request: String,
    pub observed: u64,
    pub outcome: Outcome,
    pub data: Data,
}

impl Import {
    pub fn validate(&self) -> Result<(), crate::tencent::Failure> {
        self.body().map(|_| ())
    }
}
