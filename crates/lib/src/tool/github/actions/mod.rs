use super::{
    Call, Cause, Client, Error, Origin, Receipt, node,
    pull::{Principal, sha},
    reference, valid,
};
mod archive;
mod artifact;
pub(super) mod command;
mod jobs;
mod launch;
mod logs;
pub mod release;
mod status;
pub use status::{Combined, Signal, Status, Verdict};

pub use archive::{Archive, Download};
pub use artifact::{Artifact, Artifacts, Inventory, Producer};
pub use jobs::{Job, Jobs, Listing, Step};
pub use launch::{Dispatch, Launch};
pub use logs::{INLINE, Log, Transcript};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Filter {
    head: String,
    page: u32,
    size: u8,
}

impl Filter {
    pub fn new(head: &str, page: u32, size: u8) -> Result<Self, Error> {
        if !sha(head) || page == 0 || !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            head: head.to_ascii_lowercase(),
            page,
            size,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Checks {
    #[serde(rename = "total_count")]
    pub total: u32,
    #[serde(rename = "check_runs")]
    pub rows: Vec<Check>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Check {
    pub id: u64,
    pub name: String,
    #[serde(rename = "head_sha")]
    pub head: String,
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "details_url")]
    pub url: Option<String>,
    pub app: Option<App>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct App {
    pub id: u64,
    pub slug: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct Invocation {
    id: u64,
    attempt: Option<u32>,
}

impl Invocation {
    pub fn new(id: u64, attempt: Option<u32>) -> Result<Self, Error> {
        if id == 0 || attempt == Some(0) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self { id, attempt })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Run {
    pub id: u64,
    #[serde(rename = "workflow_id")]
    pub workflow: u64,
    #[serde(rename = "run_attempt")]
    pub attempt: u32,
    #[serde(rename = "head_sha")]
    pub head: String,
    #[serde(rename = "head_branch")]
    pub branch: Option<String>,
    #[serde(rename = "html_url")]
    pub url: String,
    pub event: String,
    #[serde(rename = "display_title", skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Principal>,
    #[serde(rename = "triggering_actor", skip_serializing_if = "Option::is_none")]
    pub initiator: Option<Principal>,
    pub path: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub repository: Origin,
    #[serde(rename = "head_repository", skip_serializing_if = "Option::is_none")]
    pub source: Option<Origin>,
}

impl Client {
    pub fn checks(
        &self,
        owner: &str,
        name: &str,
        filter: &Filter,
    ) -> Result<Receipt<Checks>, Error> {
        let repo = repository(owner, name)?;
        let path = format!(
            "/repos/{repo}/commits/{}/check-runs?per_page={}&page={}",
            filter.head, filter.size, filter.page
        );
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Checks = serde_json::from_value(value).ok()?;
            confirmed(filter, &found).then_some(found)
        })
    }

    pub fn workflow(
        &self,
        owner: &str,
        name: &str,
        invocation: Invocation,
    ) -> Result<Receipt<Run>, Error> {
        let repo = repository(owner, name)?;
        let mut path = format!("/repos/{repo}/actions/runs/{}", invocation.id);
        if let Some(attempt) = invocation.attempt {
            path.push_str(&format!("/attempts/{attempt}"));
        }
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Run = serde_json::from_value(value).ok()?;
            invocation.confirmed(&repo, &found).then_some(found)
        })
    }
}

impl Invocation {
    fn confirmed(self, repo: &str, found: &Run) -> bool {
        if found.id != self.id || found.attempt == 0 || !sha(&found.head) {
            return false;
        }
        self.attempt.is_none_or(|attempt| found.attempt == attempt)
            && found.repository.name.eq_ignore_ascii_case(repo)
    }
}

fn confirmed(filter: &Filter, found: &Checks) -> bool {
    if found.rows.len() > usize::from(filter.size)
        || found.rows.len() as u64 > u64::from(found.total)
    {
        return false;
    }
    found
        .rows
        .iter()
        .all(|item| item.id != 0 && item.head.eq_ignore_ascii_case(&filter.head))
}

fn repository(owner: &str, name: &str) -> Result<String, Error> {
    if !valid(owner, false) || !valid(name, true) {
        return Err(Error::local(Cause::Input));
    }
    Ok(format!("{owner}/{name}"))
}
