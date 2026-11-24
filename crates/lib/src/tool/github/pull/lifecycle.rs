use super::{
    Call, Cause, Client, Error, Pull, Receipt, State, Target, branch, confirmed, repository, sha,
};
use crate::github::{Number, field, graph, node, reference};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use url::form_urlencoded::Serializer;

const READINESS: &str = "query RunsealReadiness($owner:String!,$name:String!,$number:Int!) { repository(owner:$owner,name:$name) { id nameWithOwner pullRequest(number:$number) { id number head:headRefOid state:mergeStateStatus } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Readiness {
    pub id: String,
    pub number: u32,
    pub head: String,
    pub state: String,
}

#[derive(Debug)]
pub enum Update {
    Body(String),
    Close,
}

#[derive(Debug)]
pub struct Selection {
    head: String,
    base: Option<String>,
    state: State,
    window: Number,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Listing {
    pub page: u32,
    pub rows: Vec<Pull>,
    pub next: Option<u32>,
}

impl Client {
    pub fn readiness(&self, target: &Target) -> Result<Receipt<Readiness>, Error> {
        self.wire.send(
            &self.bearer,
            Call::Query(graph(READINESS, target.variables())),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let name = repo.get("nameWithOwner")?.as_str()?;
                let id = repo.get("id")?.as_str()?;
                if !name.eq_ignore_ascii_case(&target.name()) || !node(id) {
                    return None;
                }
                let found: Readiness = field(repo, "/pullRequest")?;
                let identity = found.number == target.number() && node(&found.id);
                let state = !found.state.is_empty()
                    && found
                        .state
                        .bytes()
                        .all(|byte| byte.is_ascii_uppercase() || byte == b'_');
                (identity && state && sha(&found.head)).then_some(found)
            },
        )
    }

    pub fn revise(&self, target: &Target, update: &Update) -> Result<Receipt<Pull>, Error> {
        let input = update.input().ok_or_else(|| Error::local(Cause::Input))?;
        let path = format!("/repos/{}/pulls/{}", target.name(), target.number());
        self.wire
            .send(&self.bearer, Call::Patch(&path, input), |value| {
                let found: Pull = serde_json::from_value(value).ok()?;
                (confirmed(target, &found) && update.matches(&found)).then_some(found)
            })
    }

    pub fn listing(
        &self,
        owner: &str,
        name: &str,
        selection: &Selection,
    ) -> Result<Receipt<Listing>, Error> {
        let repo = repository(owner, name)?;
        let query = selection.query(owner);
        let path = format!("/repos/{repo}/pulls?{query}");
        self.wire
            .inspect(&self.bearer, Call::Source(&path), |value, response| {
                let rows: Vec<Pull> = serde_json::from_value(value).ok()?;
                let link = response.headers.get("link").map(|value| value.as_str());
                let next = selection.window.next(link, rows.len())?;
                let mut seen = BTreeSet::new();
                rows.iter()
                    .all(|row| selection.matches(&repo, row) && seen.insert(row.number))
                    .then_some(Listing {
                        page: selection.window.page,
                        rows,
                        next,
                    })
            })
    }
}

impl Update {
    fn input(&self) -> Option<Value> {
        match self {
            Self::Body(body) if body.len() <= 65_536 => Some(json!({"body":body})),
            Self::Body(_) => None,
            Self::Close => Some(json!({"state":"closed"})),
        }
    }

    fn matches(&self, pull: &Pull) -> bool {
        match self {
            Self::Body(body) => pull.body.as_deref() == Some(body),
            Self::Close => {
                pull.state == "closed" && pull.merged == Some(false) && pull.landed.is_none()
            }
        }
    }
}

impl Selection {
    pub fn new(
        head: &str,
        base: Option<&str>,
        state: State,
        window: Number,
    ) -> Result<Self, Error> {
        if !reference(head) || base.is_some_and(|name| !reference(name) || name == head) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            head: head.into(),
            base: base.map(str::to_owned),
            state,
            window,
        })
    }

    fn query(&self, owner: &str) -> String {
        let mut query = Serializer::new(String::new());
        query.append_pair("head", &format!("{owner}:{}", self.head));
        if let Some(base) = &self.base {
            query.append_pair("base", base);
        }
        query.append_pair("state", self.state.name());
        query.append_pair("per_page", &self.window.size.to_string());
        query.append_pair("page", &self.window.page.to_string());
        query.finish()
    }

    fn matches(&self, repo: &str, pull: &Pull) -> bool {
        let identity = node(&pull.node) && pull.number > 0 && pull.id > 0;
        let state = match self.state {
            State::All => matches!(pull.state.as_str(), "open" | "closed"),
            state => pull.state == state.name(),
        };
        let names = pull.head.branch == self.head
            && self
                .base
                .as_ref()
                .is_none_or(|base| pull.base.branch == *base);
        let branches = names && branch(repo, &pull.head) && branch(repo, &pull.base);
        identity && state && branches
    }
}
