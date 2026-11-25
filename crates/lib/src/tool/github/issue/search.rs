use super::{Call, Cause, Client, Error, Label, Receipt, Window, field, graph, node, valid};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const QUERY: &str = "query RunsealSearch($query:String!,$size:Int!,$after:String) { search(query:$query,type:ISSUE,first:$size,after:$after) { total:issueCount page:pageInfo { more:hasNextPage cursor:endCursor } rows:nodes { ... on Issue { id number url title body state updated:updatedAt kind:issueType { name } repository { name:nameWithOwner } } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Search {
    pub total: u32,
    pub rows: Vec<Entry>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub id: String,
    pub number: u32,
    pub url: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub updated: String,
    pub kind: Option<Label>,
    pub repository: Repository,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub name: String,
}

#[derive(Deserialize)]
struct Info {
    more: bool,
    cursor: Option<String>,
}

impl Client {
    pub fn search(&self, query: &str, window: &Window) -> Result<Receipt<Search>, Error> {
        let query = query.trim();
        if query.is_empty() || query.len() > 1024 || query.contains('\0') {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"query":query,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Query(graph(QUERY, variables)),
            |value| decode(&value, window),
        )
    }
}

fn decode(value: &Value, window: &Window) -> Option<Search> {
    let scope = value.pointer("/data/search")?;
    let total: u32 = field(scope, "/total")?;
    let info: Info = field(scope, "/page")?;
    let entries = scope.get("rows")?.as_array()?;
    if entries.len() > usize::from(window.size) {
        return None;
    }
    let mut rows = Vec::with_capacity(entries.len());
    let mut seen = BTreeSet::new();
    for entry in entries {
        let row = Entry::deserialize(entry).ok()?;
        if !row.valid() || !seen.insert(row.id.clone()) {
            return None;
        }
        rows.push(row);
    }
    let next = if info.more {
        let cursor = info.cursor.filter(|value| advance(value, window))?;
        Some((!rows.is_empty()).then_some(cursor)?)
    } else {
        None
    };
    Some(Search { total, rows, next })
}

fn advance(value: &str, window: &Window) -> bool {
    if value.is_empty() || value.len() > 4096 {
        return false;
    }
    value.bytes().all(|byte| byte.is_ascii_graphic()) && window.after.as_deref() != Some(value)
}

impl Entry {
    fn valid(&self) -> bool {
        if !node(&self.id) || self.number == 0 || self.updated.is_empty() {
            return false;
        }
        let Some((owner, name)) = self.repository.name.split_once('/') else {
            return false;
        };
        if !valid(owner, false) || !valid(name, true) {
            return false;
        }
        let expected = format!(
            "https://github.com/{}/issues/{}",
            self.repository.name, self.number
        );
        self.url.eq_ignore_ascii_case(&expected) && matches!(self.state.as_str(), "OPEN" | "CLOSED")
    }
}
