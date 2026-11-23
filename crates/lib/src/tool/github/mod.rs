mod command;
mod model;
mod wire;

pub(crate) use command::{call, run};
pub use model::{Cause, Error, Evidence, Issue, Outcome, Project, Receipt, Repository};

use crate::custody::Private;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::Path;
use wire::{Call, Wire};
use zeroize::Zeroizing;

const PROJECT: &str = "query RunsealProject($owner: String!, $number: Int!) { organization(login: $owner) { login projectV2(number: $number) { id number title } } }";
const CLOSE: &str = "mutation RunsealClose($issue: ID!) { closeIssue(input: {issueId: $issue, stateReason: COMPLETED}) { issue { id state reason: stateReason } } }";

pub struct Client {
    wire: Wire,
    bearer: Zeroizing<String>,
}

impl Client {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let file = Zeroizing::new(
            Private::current()
                .read(path, 4096)
                .map_err(|_| Error::local(Cause::Custody))?,
        );
        let token = std::str::from_utf8(&file).map_err(|_| Error::local(Cause::Custody))?;
        Self::token(token)
    }

    pub fn token(token: &str) -> Result<Self, Error> {
        let token = token.trim();
        if token.is_empty() || token.len() > 4096 || !token.bytes().all(symbol) {
            return Err(Error::local(Cause::Custody));
        }
        Ok(Self {
            wire: Wire::new("https://api.github.com".into()),
            bearer: Zeroizing::new(format!("Bearer {token}")),
        })
    }

    pub fn repository(&self, owner: &str, name: &str) -> Result<Receipt<Repository>, Error> {
        if !valid(owner, false) || !valid(name, true) {
            return Err(Error::local(Cause::Input));
        }
        let expected = format!("{owner}/{name}");
        self.wire.send(
            &self.bearer,
            Call::Rest(&format!("/repos/{expected}")),
            |value| {
                let repo: Repository = serde_json::from_value(value).ok()?;
                repo.name.eq_ignore_ascii_case(&expected).then_some(repo)
            },
        )
    }

    pub fn project(&self, owner: &str, number: u32) -> Result<Receipt<Project>, Error> {
        if !valid(owner, false) || number == 0 || number > i32::MAX as u32 {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner, "number":number});
        self.wire.send(
            &self.bearer,
            Call::Query(graph(PROJECT, variables)),
            |value| {
                let login = value.pointer("/data/organization/login")?.as_str()?;
                if !login.eq_ignore_ascii_case(owner) {
                    return None;
                }
                let found: Project = field(&value, "/data/organization/projectV2")?;
                (found.number == number && !found.id.is_empty()).then_some(found)
            },
        )
    }

    pub fn close(&self, issue: &str) -> Result<Receipt<Issue>, Error> {
        if !node(issue) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"issue":issue});
        self.wire.send(
            &self.bearer,
            Call::Mutation(graph(CLOSE, variables)),
            |value| {
                let found: Issue = field(&value, "/data/closeIssue/issue")?;
                (found.id == issue && found.state == "CLOSED" && found.reason == "COMPLETED")
                    .then_some(found)
            },
        )
    }
}

fn graph(query: &str, variables: Value) -> Value {
    json!({"query":query, "variables":variables})
}

fn field<T: DeserializeOwned>(value: &Value, pointer: &str) -> Option<T> {
    T::deserialize(value.pointer(pointer)?).ok()
}

fn symbol(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn valid(name: &str, dots: bool) -> bool {
    if name.is_empty() || name.len() > 128 || matches!(name, "." | "..") {
        return false;
    }
    name.bytes()
        .all(|byte| symbol(byte) || (dots && byte == b'.'))
}

fn node(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| symbol(byte) || matches!(byte, b'+' | b'/' | b'='))
}

#[cfg(test)]
mod tests;
