mod actions;
mod command;
mod issue;
mod model;
mod page;
mod project;
mod pull;
mod wire;

pub use actions::release;
pub use actions::{
    App, Archive, Artifact, Artifacts, Check, Checks, Combined, Dispatch, Download, Event, Filter,
    Inventory, Invocation, Job, Jobs, Launch, Listing, Log, Producer, Run, Signal, Status, Step,
    Transcript, Trigger, Verdict,
};
pub(crate) use command::{call, run};
pub use issue::acceptance;
pub use issue::attached;
pub use issue::classify;
pub use issue::discussion;
pub use issue::phase;
pub use issue::records;
pub use issue::review;
pub use issue::search;
pub use issue::snapshot;
pub use issue::{Comment, Draft, Identity, Kind, Label, Link, Related, Relation, Ticket};
pub use model::{
    Cause, Creation, Error, Evidence, Issue, Outcome, Project, Receipt, Repository, Target,
    Visibility,
};
pub use page::{Number, Page, Window};
pub use project::activity;
pub use project::board;
pub use project::catalogue;
pub use project::columns;
pub use project::inventory;
pub use project::view;
pub use project::{
    Acknowledgement, Action, Alternative, Calendar, Change, Choice, Color, Confirmation, Content,
    Definition, Descriptor, Entry, Field, Item, Iteration, Organization, Overview, Period,
    Schedule, Schema, Seat, Settings, Shape, Structure,
};
pub use pull::{
    Annotation, Blob, Branch, Closing, Commit, Compare, Directory, Edit, File, Head, Manifest,
    Member, Merge, Object, Origin, Permission, Pointer, Principal, Proposal, Pull, Reference,
    Revision, Search, Squash, State, Tree,
};
pub use pull::{lifecycle, lineage, rollup, tags};
pub use wire::{Observation, Sink, observe};

use crate::custody::Private;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::Path;
use wire::{Call, Wire};
use zeroize::Zeroizing;

const ORIGIN: &str = "https://api.github.com";
const PROJECT: &str = "query RunsealProject($owner: String!, $number: Int!) { organization(login: $owner) { login projectV2(number: $number) { id number title } } }";
const CLOSE: &str = "mutation RunsealClose($issue: ID!) { closeIssue(input: {issueId: $issue, stateReason: COMPLETED}) { issue { id state reason: stateReason repository { id } } } }";

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

    pub fn reader(input: impl std::io::Read) -> Result<Self, Error> {
        use std::io::Read;
        let mut held = Zeroizing::new(Vec::new());
        input
            .take(4097)
            .read_to_end(&mut held)
            .map_err(|_| Error::local(Cause::Custody))?;
        if held.len() > 4096 {
            return Err(Error::local(Cause::Custody));
        }
        let text = std::str::from_utf8(&held).map_err(|_| Error::local(Cause::Custody))?;
        let line = text.strip_suffix('\n').unwrap_or(text);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.contains(['\n', '\r']) {
            return Err(Error::local(Cause::Custody));
        }
        Self::token(line)
    }

    pub fn token(token: &str) -> Result<Self, Error> {
        let token = token.trim();
        if token.is_empty() || token.len() > 4096 || !token.bytes().all(credential) {
            return Err(Error::local(Cause::Custody));
        }
        Ok(Self {
            wire: Wire::new(ORIGIN.into()),
            bearer: Zeroizing::new(format!("Bearer {token}")),
        })
    }

    pub fn loopback(token: &str, address: std::net::SocketAddr) -> Result<Self, Error> {
        if !address.ip().is_loopback() {
            return Err(Error::local(Cause::Input));
        }
        let mut client = Self::token(token)?;
        client.wire = Wire::new(format!("http://{address}"));
        Ok(client)
    }

    pub fn timeout(mut self, seconds: u64) -> Result<Self, Error> {
        if !(1..=300).contains(&seconds) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.timeout(seconds);
        Ok(self)
    }

    pub fn roots(mut self, pem: &[u8]) -> Result<Self, Error> {
        if self.wire.origin != ORIGIN {
            return Err(Error::local(Cause::Input));
        }
        self.wire.roots(pem)?;
        Ok(self)
    }

    pub fn bundle(self, path: &Path) -> Result<Self, Error> {
        let pem = Zeroizing::new(
            Private::current()
                .read(path, wire::LIMIT)
                .map_err(|_| Error::local(Cause::Custody))?,
        );
        self.roots(&pem)
    }

    pub fn observer(mut self, sink: Sink) -> Self {
        self.wire.observer(sink);
        self
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
                if repo.id == 0 || !repo.name.eq_ignore_ascii_case(&expected) {
                    return None;
                }
                repo.owner
                    .as_ref()
                    .is_none_or(|actor| actor.matches(owner))
                    .then_some(repo)
            },
        )
    }

    pub fn establish(&self, owner: &str, request: &Creation) -> Result<Receipt<Repository>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/orgs/{owner}/repos");
        let body = request.body();
        self.wire
            .submit(&self.bearer, Call::Post(&path, body), |value, response| {
                if response.status != 201 {
                    return None;
                }
                let found: Repository = serde_json::from_value(value).ok()?;
                request.matches(owner, &found).then_some(found)
            })
    }

    pub fn project(&self, owner: &str, number: u32) -> Result<Receipt<Project>, Error> {
        let variables = project::variables(owner, number)?;
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
                if found.id != issue || !node(&found.repository.id) {
                    return None;
                }
                (found.state == "CLOSED" && found.reason == "COMPLETED").then_some(found)
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

fn credential(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~+/=".contains(&byte)
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

fn reference(value: &str) -> bool {
    if value.len() > 1024 || value.contains("..") {
        return false;
    }
    value.split('/').all(|part| valid(part, true))
}

#[cfg(test)]
mod tests;
