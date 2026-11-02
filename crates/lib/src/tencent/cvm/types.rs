use super::{Import, Instance, Key, Price, Quota, Status};
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

pub type Receipt = crate::tencent::Receipt<Data>;

impl Import {
    pub fn validate(&self) -> Result<(), crate::tencent::Failure> {
        self.body().map(|_| ())
    }
}
