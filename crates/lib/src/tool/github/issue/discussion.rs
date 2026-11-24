use super::records::{Author, login, nullable};
use crate::github::{
    Call, Cause, Client, Error, Identity, Number, Principal, Receipt, Target, field, graph, node,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

const COMPLETE: &str = "mutation RunsealComplete($input:UpdateIssueInput!) { updateIssue(input:$input) { issue { id body state reason:stateReason repository { id } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub id: u64,
    #[serde(rename = "node_id")]
    pub node: String,
    pub body: String,
    #[serde(rename = "html_url")]
    pub url: String,
    #[serde(rename = "issue_url")]
    pub issue: String,
    #[serde(deserialize_with = "nullable")]
    pub user: Option<Author>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Document {
    #[serde(flatten)]
    pub entry: Entry,
    pub author: Principal,
    #[serde(rename = "updated_at")]
    pub updated: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Discussion {
    pub page: u32,
    pub rows: Vec<Entry>,
    pub next: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Completion {
    pub id: String,
    pub body: String,
    pub state: String,
    pub reason: String,
    pub repository: Identity,
}

impl Client {
    pub fn discussion(
        &self,
        target: &Target,
        window: Number,
    ) -> Result<Receipt<Discussion>, Error> {
        let scope = format!("/repos/{}/issues/{}", target.name(), target.number());
        let path = format!(
            "{scope}/comments?per_page={}&page={}",
            window.size, window.page
        );
        self.wire
            .inspect(&self.bearer, Call::Source(&path), |value, response| {
                let rows: Vec<Entry> = serde_json::from_value(value).ok()?;
                let link = response.headers.get("link").map(|value| value.as_str());
                let next = window.next(link, rows.len())?;
                let mut ids = BTreeSet::new();
                let mut nodes = BTreeSet::new();
                rows.iter()
                    .all(|row| {
                        row.matches(target, &scope) && ids.insert(row.id) && nodes.insert(&row.node)
                    })
                    .then_some(Discussion {
                        page: window.page,
                        rows,
                        next,
                    })
            })
    }

    pub fn entry(&self, target: &Target, id: u64) -> Result<Receipt<Entry>, Error> {
        self.remark(target, id, |row, _| Some(row))
    }

    pub fn document(&self, target: &Target, id: u64) -> Result<Receipt<Document>, Error> {
        self.remark(target, id, Document::read)
    }

    pub fn lookup(&self, owner: &str, name: &str, id: u64) -> Result<Receipt<Document>, Error> {
        let repo = crate::github::pull::repository(owner, name)?;
        if id == 0 {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repo}/issues/comments/{id}");
        let scope = format!("/repos/{repo}/issues");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let entry = Entry::deserialize(&value).ok()?;
            let (origin, number) = entry.issue.rsplit_once('/')?;
            if !origin.eq_ignore_ascii_case(&format!("https://api.github.com{scope}")) {
                return None;
            }
            let target = Target::new(owner, name, number.parse().ok()?).ok()?;
            if entry.id != id || !entry.matches(&target, &format!("{scope}/{number}")) {
                return None;
            }
            Document::read(entry, &value)
        })
    }

    fn remark<T>(
        &self,
        target: &Target,
        id: u64,
        verify: impl Fn(Entry, &serde_json::Value) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        if id == 0 {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{}/issues/comments/{id}", target.name());
        let scope = format!("/repos/{}/issues/{}", target.name(), target.number());
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let row = Entry::deserialize(&value).ok()?;
            if row.id != id || !row.matches(target, &scope) {
                return None;
            }
            verify(row, &value)
        })
    }

    pub fn complete(&self, issue: &str, body: &str) -> Result<Receipt<Completion>, Error> {
        if !node(issue) || body.len() > 65_536 {
            return Err(Error::local(Cause::Input));
        }
        let input = json!({"id":issue,"body":body,"stateInput":{"value":"CLOSED","stateReason":"COMPLETED"}});
        self.wire.send(
            &self.bearer,
            Call::Mutation(graph(COMPLETE, json!({"input":input}))),
            |value| {
                let found: Completion = field(&value, "/data/updateIssue/issue")?;
                let identity = found.id == issue && node(&found.repository.id);
                let completed = found.state == "CLOSED" && found.reason == "COMPLETED";
                (identity && completed && found.body == body).then_some(found)
            },
        )
    }
}

impl Entry {
    fn matches(&self, target: &Target, scope: &str) -> bool {
        let issue = format!("https://api.github.com{scope}");
        let url = format!(
            "https://github.com/{}/issues/{}#issuecomment-{}",
            target.name(),
            target.number(),
            self.id
        );
        let routes = self.url.eq_ignore_ascii_case(&url) && self.issue.eq_ignore_ascii_case(&issue);
        let identity = self.id > 0 && node(&self.node);
        let user = self
            .user
            .as_ref()
            .is_none_or(|user| user.id > 0 && login(&user.login));
        routes && identity && user
    }
}

impl Document {
    fn read(entry: Entry, value: &serde_json::Value) -> Option<Self> {
        let user = entry.user.as_ref()?;
        let author: Principal = field(value, "/user")?;
        let updated = value.get("updated_at")?.as_str()?;
        let identity = user.id == author.id && author.matches(&user.login);
        if !identity || !matches!(author.kind.as_str(), "User" | "Bot") || updated.is_empty() {
            return None;
        }
        Some(Self {
            entry,
            author,
            updated: updated.into(),
        })
    }
}
