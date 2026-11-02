use super::{Instance, Key, Lookup};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Extension {
    #[serde(rename = "activity-labels")]
    Labels,
    #[serde(rename = "blueprint-image-url")]
    Image,
    #[serde(rename = "display-area")]
    Area,
    #[serde(rename = "display-label")]
    Display,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

#[derive(Clone, Debug, Serialize)]
pub struct Opaque {
    pub field: Extension,
    pub shape: Shape,
}

#[derive(Clone, Debug, Serialize)]
pub struct Schema {
    pub fields: Vec<String>,
    pub omitted: u64,
}

impl Schema {
    pub(super) fn known(&self) -> bool {
        self.fields.is_empty() && self.omitted == 0
    }
}

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
