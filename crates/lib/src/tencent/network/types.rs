use super::super::{Failure, Identity, Page};
use super::Rule;
use serde::Serialize;

pub use super::super::{Error, Outcome};

#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub identity: Identity,
    pub service: String,
    pub version: String,
    pub action: String,
    pub region: String,
    pub observed: u64,
    pub request: String,
    pub outcome: Outcome,
    pub data: Data,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Data {
    Created {
        entry: Entry,
    },
    Entries {
        page: Page<Entry>,
    },
    Policies {
        version: String,
        ingress: Vec<Rule>,
        egress: Vec<Rule>,
    },
    Ack,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    Vpc {
        id: String,
        name: String,
        cidr: String,
        default: Option<bool>,
    },
    Subnet {
        id: String,
        name: String,
        vpc: String,
        zone: String,
        cidr: String,
        default: Option<bool>,
    },
    Group {
        id: String,
        name: String,
        description: String,
        project: String,
        default: Option<bool>,
    },
}

impl Entry {
    pub(super) fn id(&self) -> &str {
        match self {
            Self::Vpc { id, .. } | Self::Subnet { id, .. } | Self::Group { id, .. } => id,
        }
    }
}

impl Data {
    pub(super) fn regional(&self, region: &str) -> Result<(), Failure> {
        let entries = match self {
            Self::Created { entry } => std::slice::from_ref(entry),
            Self::Entries { page } => page.items.as_slice(),
            _ => &[],
        };
        for entry in entries {
            if let Entry::Subnet { zone, .. } = entry {
                let prefix = format!("{region}-");
                let suffix = zone
                    .strip_prefix(&prefix)
                    .filter(|suffix| !suffix.is_empty())
                    .ok_or_else(|| {
                        Failure::new(
                            super::super::Class::Response,
                            "subnet response crosses the requested region",
                        )
                    })?;
                if !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(Failure::new(
                        super::super::Class::Response,
                        "invalid subnet response zone",
                    ));
                }
            }
        }
        Ok(())
    }
}
