use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Unsent,
    Refused,
    Unknown,
    Observed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cause {
    Input,
    Custody,
    Network,
    Read,
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
    #[serde(rename = "full_name")]
    pub name: String,
    pub private: bool,
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
}
