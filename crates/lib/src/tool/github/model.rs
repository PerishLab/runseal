use super::{Identity, Principal, valid};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Unsent,
    Refused,
    Unknown,
    Observed,
    Accepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cause {
    Input,
    Custody,
    Network,
    Read,
    Output,
    Http,
    Limited,
    Graphql,
    Shape,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Evidence {
    pub attempts: u8,
    pub status: Option<u16>,
    pub request: Option<String>,
    pub wait: Option<u64>,
    pub reset: Option<u64>,
    pub partial: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Error {
    pub outcome: Outcome,
    pub cause: Cause,
    pub evidence: Evidence,
}

impl Error {
    pub(crate) fn local(cause: Cause) -> Self {
        Self {
            cause,
            outcome: Outcome::Unsent,
            evidence: Evidence::default(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, format: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            format,
            "GitHub request {:?} ({:?})",
            self.outcome, self.cause
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct Receipt<T> {
    pub outcome: Outcome,
    pub evidence: Evidence,
    pub value: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub id: u64,
    #[serde(rename = "node_id", skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(rename = "full_name")]
    pub name: String,
    pub private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Principal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "default_branch")]
    pub branch: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub id: String,
    pub number: u32,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Issue {
    pub id: String,
    pub state: String,
    pub reason: String,
    pub repository: Identity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    owner: String,
    name: String,
    number: u32,
}

impl Target {
    pub fn new(owner: &str, name: &str, number: u32) -> Result<Self, Error> {
        if !valid(owner, false) || !valid(name, true) {
            return Err(Error::local(Cause::Input));
        }
        if !(1..=i32::MAX as u32).contains(&number) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            owner: owner.into(),
            name: name.into(),
            number,
        })
    }

    pub(super) fn name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub(super) fn number(&self) -> u32 {
        self.number
    }

    pub(super) fn variables(&self) -> Value {
        json!({"owner":self.owner,"name":self.name,"number":self.number})
    }

    pub(super) fn issue(&self, value: &Value) -> Option<()> {
        let repository = value.pointer("/data/repository/nameWithOwner")?.as_str()?;
        if !repository.eq_ignore_ascii_case(&self.name()) {
            return None;
        }
        let issue = value.pointer("/data/repository/issue")?;
        (issue.get("number")?.as_u64()? == u64::from(self.number)).then_some(())
    }
}
