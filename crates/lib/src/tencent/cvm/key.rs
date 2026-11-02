use super::{Failure, input, names::Names};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub name: String,
    pub project: u64,
    pub public: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Key {
    pub id: String,
    pub name: String,
    pub project: u64,
    pub public: String,
    pub instances: Vec<String>,
}

impl Import {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        Names::text(&self.name, 25)?;
        if !self
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || self.project > i64::MAX as u64
        {
            return Err(input());
        }
        public(&self.public)?;
        Ok(json!({"KeyName":self.name,"ProjectId":self.project,"PublicKey":self.public}))
    }
}

pub(super) fn public(value: &str) -> Result<(), Failure> {
    crate::tencent::api::key::public(value).map(|_| ())
}
