use super::{Instance, Key, Lookup};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct Label(String);

impl Label {
    pub fn value(&self) -> &str {
        &self.0
    }

    pub(super) fn parse(value: &str, known: &[&str]) -> Self {
        Self(
            if known.contains(&value) {
                value
            } else {
                "UNKNOWN"
            }
            .into(),
        )
    }

    pub(super) fn known(&self) -> bool {
        self.value() != "UNKNOWN"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Windows,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Operating,
    Success,
    Failed,
    Unavailable,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct Page {
    pub entries: Vec<Key>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
    pub next: Option<u32>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Data {
    Instance(Lookup<Box<Instance>>),
    Key(Lookup<Key>),
    Keys(Page),
}

pub type Receipt = crate::tencent::Receipt<Data>;
