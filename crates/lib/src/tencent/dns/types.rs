use super::super::{Failure, Identity, Outcome, Page};
use super::{Class, input};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Zone {
    Name(String),
    Id(u64),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Reference {
    pub domain: Option<u64>,
    pub record: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Remote {
    pub account: String,
    pub domain: u64,
    pub record: u64,
}

impl Reference {
    pub fn complete(&self, account: &str) -> Result<Remote, Failure> {
        if account.starts_with('0')
            || !account.bytes().all(|byte| byte.is_ascii_digit())
            || account.parse::<u64>().ok().is_none_or(|value| value == 0)
        {
            return Err(input());
        }
        let domain = self.domain.filter(|value| *value != 0).ok_or_else(|| {
            Failure::new(
                Class::Binding,
                "DNS reference lacks DomainId; explicitly qualify the domain before binding",
            )
        })?;
        if self.record == 0 {
            return Err(input());
        }
        Ok(Remote {
            account: account.into(),
            domain,
            record: self.record,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Kind {
    A,
    Aaaa,
    Cname,
    Txt,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aaaa => "AAAA",
            Self::Cname => "CNAME",
            Self::Txt => "TXT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Fields {
    pub name: String,
    pub kind: Kind,
    pub value: String,
    pub line: String,
    pub ttl: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Domain {
    pub account: Option<String>,
    pub id: u64,
    pub name: String,
    pub status: String,
    pub ttl: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub reference: Reference,
    pub name: String,
    pub kind: String,
    pub value: String,
    pub line: String,
    pub route: String,
    pub ttl: u64,
    pub status: Status,
    pub weight: Option<u64>,
    pub mx: Option<u64>,
    pub default: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Data {
    Domain(Domain),
    Record(Entry),
    Records(Page<Entry>),
    Acknowledged(Reference),
}

#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub identity: Identity,
    pub service: String,
    pub version: String,
    pub action: String,
    pub region: Option<String>,
    pub zone: Zone,
    pub request: String,
    pub observed: u64,
    pub outcome: Outcome,
    pub data: Data,
}
