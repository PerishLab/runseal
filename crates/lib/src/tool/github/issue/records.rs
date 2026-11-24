use crate::github::{
    Call, Cause, Client, Error, Label, Receipt, Target, field, graph, node, valid,
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const VIEWER: &str = "query RunsealViewer { viewer { id databaseId login } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Viewer {
    pub id: String,
    #[serde(rename = "databaseId")]
    pub number: u64,
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Author {
    pub id: u64,
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Record {
    #[serde(rename = "node_id")]
    pub node: String,
    pub number: u32,
    #[serde(rename = "html_url")]
    pub url: String,
    #[serde(rename = "repository_url")]
    pub repository: String,
    pub title: String,
    #[serde(deserialize_with = "nullable")]
    pub body: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated: String,
    pub state: State,
    #[serde(rename = "state_reason", deserialize_with = "nullable")]
    pub reason: Option<String>,
    #[serde(rename = "type", deserialize_with = "nullable")]
    pub kind: Option<Label>,
    #[serde(deserialize_with = "nullable")]
    pub user: Option<Author>,
    pub labels: Vec<Label>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Open,
    Closed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Listing {
    pub repository: String,
    pub page: u32,
    pub rows: Vec<Record>,
    pub next: Option<u32>,
}

impl Client {
    pub fn viewer(&self) -> Result<Receipt<Viewer>, Error> {
        self.wire.send(
            &self.bearer,
            Call::Query(graph(VIEWER, json!({}))),
            |value| {
                let viewer: Viewer = field(&value, "/data/viewer")?;
                (node(&viewer.id) && viewer.number > 0 && login(&viewer.login)).then_some(viewer)
            },
        )
    }

    pub fn ticket(&self, target: &Target) -> Result<Receipt<Record>, Error> {
        let path = format!("/repos/{}/issues/{}", target.name(), target.number());
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            exact(value, target)
        })
    }

    pub fn issues(
        &self,
        owner: &str,
        name: &str,
        page: u32,
        size: u8,
    ) -> Result<Receipt<Listing>, Error> {
        let target = Target::new(owner, name, 1)?;
        if page == 0 || !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        let repository = target.name();
        let path = format!("/repos/{repository}/issues?state=open&per_page={size}&page={page}");
        self.wire
            .inspect(&self.bearer, Call::Source(&path), |value, response| {
                let entries = value.as_array()?;
                if entries.len() > usize::from(size) {
                    return None;
                }
                let more = match response.headers.get("link") {
                    Some(value) => super::super::page::continuation(value)?,
                    None => false,
                };
                let next = if more {
                    if entries.is_empty() {
                        return None;
                    }
                    Some(page.checked_add(1)?)
                } else {
                    None
                };
                let rows = inventory(entries, &repository)?;
                Some(Listing {
                    repository: repository.clone(),
                    page,
                    rows,
                    next,
                })
            })
    }
}

fn exact(value: Value, target: &Target) -> Option<Record> {
    if value.get("pull_request").is_some() {
        return None;
    }
    let record: Record = serde_json::from_value(value).ok()?;
    (record.number == target.number() && record.matches(&target.name())).then_some(record)
}

fn inventory(entries: &[Value], repository: &str) -> Option<Vec<Record>> {
    let mut numbers = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    let mut rows = Vec::new();
    for entry in entries {
        if entry.get("pull_request").is_some() {
            continue;
        }
        let row = Record::deserialize(entry).ok()?;
        let identity = numbers.insert(row.number) && nodes.insert(row.node.clone());
        if !row.matches(repository) || row.state != State::Open || !identity {
            return None;
        }
        rows.push(row);
    }
    Some(rows)
}

impl Record {
    fn matches(&self, repository: &str) -> bool {
        let api = format!("https://api.github.com/repos/{repository}");
        let web = format!("https://github.com/{repository}/issues/{}", self.number);
        let routes =
            self.repository.eq_ignore_ascii_case(&api) && self.url.eq_ignore_ascii_case(&web);
        let identity = self.number > 0 && node(&self.node) && !self.updated.is_empty();
        let author = self
            .user
            .as_ref()
            .is_none_or(|actor| actor.id > 0 && login(&actor.login));
        routes && identity && author
    }
}

fn login(name: &str) -> bool {
    valid(name.strip_suffix("[bot]").unwrap_or(name), false)
}

fn nullable<'de, D: Deserializer<'de>, T: DeserializeOwned>(de: D) -> Result<Option<T>, D::Error> {
    Option::deserialize(de)
}
