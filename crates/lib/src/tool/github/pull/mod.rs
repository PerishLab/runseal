mod authority;
pub use authority::{Branch, Closing, Head, Manifest, Permission, Principal};
mod commit;
mod edit;
pub mod lifecycle;
pub mod lineage;
mod pointer;
pub mod rollup;
mod source;
pub mod tags;
pub use pointer::{Annotation, Object, Pointer, Reference};
pub use source::{Blob, Directory, File, Member};

pub use commit::{Commit, Tree};
pub use edit::Edit;

use super::{Call, Cause, Client, Error, Receipt, Target, node, reference, valid};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::form_urlencoded::Serializer;

#[derive(Debug, Deserialize, Serialize)]
pub struct Pull {
    pub id: u64,
    pub number: u32,
    #[serde(rename = "node_id")]
    pub node: String,
    #[serde(rename = "html_url")]
    pub url: String,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub draft: bool,
    pub merged: Option<bool>,
    #[serde(rename = "merge_commit_sha")]
    pub merge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "merged_at")]
    pub landed: Option<String>,
    #[serde(rename = "updated_at", skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub head: Revision,
    pub base: Revision,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Revision {
    #[serde(rename = "ref")]
    pub branch: String,
    pub sha: String,
    pub repo: Option<Origin>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Origin {
    pub id: u64,
    #[serde(rename = "node_id", skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(rename = "full_name")]
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Squash {
    sha: String,
    #[serde(rename = "commit_title", skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "commit_message", skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(rename = "merge_method")]
    method: &'static str,
}

impl Squash {
    pub fn new(head: &str, title: Option<&str>, body: Option<&str>) -> Result<Self, Error> {
        if !sha(head) || title.is_some_and(|value| value.len() > 256) {
            return Err(Error::local(Cause::Input));
        }
        if body.is_some_and(|value| value.len() > 65_536) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            sha: head.to_ascii_lowercase(),
            title: title.map(str::to_owned),
            body: body.map(str::to_owned),
            method: "squash",
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Merge {
    pub sha: String,
    pub merged: bool,
}

impl Client {
    pub fn pull(&self, target: &Target) -> Result<Receipt<Pull>, Error> {
        let path = format!("/repos/{}/pulls/{}", target.name(), target.number());
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Pull = serde_json::from_value(value).ok()?;
            (confirmed(target, &found) && found.merged.is_some()).then_some(found)
        })
    }

    pub fn squash(&self, target: &Target, input: &Squash) -> Result<Receipt<Merge>, Error> {
        let path = format!("/repos/{}/pulls/{}/merge", target.name(), target.number());
        self.wire
            .send(&self.bearer, Call::Put(&path, json!(input)), |value| {
                let found: Merge = serde_json::from_value(value).ok()?;
                (found.merged && sha(&found.sha)).then_some(found)
            })
    }
}

fn confirmed(target: &Target, found: &Pull) -> bool {
    if found.number != target.number() || !node(&found.node) {
        return false;
    }
    let Some(repo) = found.base.repo.as_ref() else {
        return false;
    };
    repo.name.eq_ignore_ascii_case(&target.name()) && sha(&found.base.sha) && sha(&found.head.sha)
}

pub(super) fn sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Serialize)]
pub struct Compare {
    head: String,
    base: String,
}

impl Compare {
    pub fn new(head: &str, base: &str) -> Result<Self, Error> {
        if !reference(head) || !reference(base) || head == base {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            head: head.into(),
            base: base.into(),
        })
    }

    fn confirmed(&self, repo: &str, found: &Pull) -> bool {
        if found.base.branch != self.base || found.head.branch != self.head {
            return false;
        }
        branch(repo, &found.base) && branch(repo, &found.head)
    }
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum State {
    Open,
    Closed,
    All,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

#[derive(Debug)]
pub struct Search {
    compare: Compare,
    state: State,
    page: u32,
    size: u8,
}

impl Search {
    pub fn new(compare: Compare, state: State, page: u32, size: u8) -> Result<Self, Error> {
        if page == 0 || !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            compare,
            state,
            page,
            size,
        })
    }

    fn query(&self, owner: &str) -> String {
        Serializer::new(String::new())
            .append_pair("head", &format!("{owner}:{}", self.compare.head))
            .append_pair("base", &self.compare.base)
            .append_pair("state", self.state.name())
            .append_pair("per_page", &self.size.to_string())
            .append_pair("page", &self.page.to_string())
            .finish()
    }

    fn matches(&self, repo: &str, rows: &[Pull]) -> bool {
        rows.len() <= usize::from(self.size) && rows.iter().all(|pull| self.admits(repo, pull))
    }

    fn admits(&self, repo: &str, pull: &Pull) -> bool {
        if !node(&pull.node) || pull.number == 0 {
            return false;
        }
        self.compare.confirmed(repo, pull)
            && (matches!(self.state, State::All) || pull.state == self.state.name())
    }
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    #[serde(flatten)]
    compare: Compare,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

impl Proposal {
    pub fn new(compare: Compare, title: &str, body: Option<&str>) -> Result<Self, Error> {
        if title.trim().is_empty()
            || title.len() > 256
            || body.is_some_and(|text| text.len() > 65_536)
        {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            compare,
            title: title.into(),
            body: body.map(str::to_owned),
        })
    }

    fn confirms(&self, repo: &str, pull: &Pull) -> bool {
        if pull.number == 0 || !node(&pull.node) || pull.title != self.title {
            return false;
        }
        self.body
            .as_ref()
            .is_none_or(|body| pull.body.as_ref() == Some(body))
            && self.compare.confirmed(repo, pull)
    }
}

impl Client {
    pub fn pulls(
        &self,
        owner: &str,
        name: &str,
        search: &Search,
    ) -> Result<Receipt<Vec<Pull>>, Error> {
        let repo = repository(owner, name)?;
        let path = format!("/repos/{repo}/pulls?{}", search.query(owner));
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let rows: Vec<Pull> = serde_json::from_value(value).ok()?;
            search.matches(&repo, &rows).then_some(rows)
        })
    }

    pub fn propose(
        &self,
        owner: &str,
        name: &str,
        proposal: &Proposal,
    ) -> Result<Receipt<Pull>, Error> {
        let repo = repository(owner, name)?;
        let path = format!("/repos/{repo}/pulls");
        self.wire
            .send(&self.bearer, Call::Post(&path, json!(proposal)), |value| {
                let found: Pull = serde_json::from_value(value).ok()?;
                proposal.confirms(&repo, &found).then_some(found)
            })
    }
}

fn branch(repo: &str, revision: &Revision) -> bool {
    revision
        .repo
        .as_ref()
        .is_some_and(|owner| owner.name.eq_ignore_ascii_case(repo))
        && sha(&revision.sha)
}

pub(super) fn repository(owner: &str, name: &str) -> Result<String, Error> {
    if !valid(owner, false) || !valid(name, true) {
        return Err(Error::local(Cause::Input));
    }
    Ok(format!("{owner}/{name}"))
}
