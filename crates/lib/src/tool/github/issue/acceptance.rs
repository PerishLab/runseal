use super::{
    Call, Cause, Client, Error, Identity, Page, Receipt, Target, Window, field, graph, node, page,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const COMMENT: &str = "id body url created:createdAt updated:updatedAt edited:lastEditedAt issue { id repository { id } }";
const HISTORY: &str =
    "id number title body url state updated:updatedAt edited:lastEditedAt repository { id }";
const EVENTS: &str = "list:timelineItems(first:$size,after:$after,itemTypes:[REOPENED_EVENT]) { total:totalCount rows:nodes { ... on ReopenedEvent { id created:createdAt } } page:pageInfo { more:hasNextPage cursor:endCursor } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Subject {
    pub id: String,
    pub title: String,
    pub body: String,
    pub url: String,
    pub state: String,
    pub updated: String,
    #[serde(deserialize_with = "edited")]
    pub edited: Option<String>,
    pub repository: Identity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub id: String,
    pub created: String,
}

#[derive(Debug, Serialize)]
pub struct History {
    pub issue: Subject,
    pub events: Page<Event>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    pub body: String,
    pub url: String,
    pub created: String,
    pub updated: String,
    #[serde(deserialize_with = "edited")]
    pub edited: Option<String>,
    pub issue: Origin,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Origin {
    pub id: String,
    pub repository: Identity,
}

#[derive(Debug)]
pub struct Pair {
    declaration: String,
    closure: String,
}

impl Pair {
    pub fn new(declaration: &str, closure: &str) -> Result<Self, Error> {
        if !node(declaration) || !node(closure) || declaration == closure {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            declaration: declaration.into(),
            closure: closure.into(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Freshness {
    #[serde(flatten)]
    pub history: History,
    pub declaration: Comment,
    pub closure: Comment,
}

impl Client {
    pub fn reopenings(&self, target: &Target, window: &Window) -> Result<Receipt<History>, Error> {
        let statement = format!(
            "query RunsealReopened($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String) {{ {} }}",
            query()
        );
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(&statement, input(target, window))),
            |value| history(target, window, &value),
        )
    }

    pub fn review(&self, issue: &str, comment: &str) -> Result<Receipt<Comment>, Error> {
        if !node(issue) || !node(comment) {
            return Err(Error::local(Cause::Input));
        }
        let query = format!(
            "query RunsealReview($comment:ID!) {{ node(id:$comment) {{ ... on IssueComment {{ {COMMENT} }} }} }}"
        );
        self.wire.send(
            &self.bearer,
            Call::Query(graph(&query, json!({"comment":comment}))),
            |value| {
                let found: Comment = field(&value, "/data/node")?;
                found.matches(issue, comment).then_some(found)
            },
        )
    }

    pub fn freshness(
        &self,
        target: &Target,
        window: &Window,
        pair: &Pair,
    ) -> Result<Receipt<Freshness>, Error> {
        let query = format!(
            "query RunsealFreshness($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String,$declaration:ID!,$closure:ID!) {{ {} declaration:node(id:$declaration) {{ ... on IssueComment {{ {COMMENT} }} }} closure:node(id:$closure) {{ ... on IssueComment {{ {COMMENT} }} }} }}",
            query()
        );
        let mut variables = input(target, window);
        variables["declaration"] = json!(pair.declaration);
        variables["closure"] = json!(pair.closure);
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(&query, variables)),
            |value| {
                let history = history(target, window, &value)?;
                let declaration = paired(
                    &value,
                    "/data/declaration",
                    &history.issue,
                    &pair.declaration,
                )?;
                let closure = paired(&value, "/data/closure", &history.issue, &pair.closure)?;
                Some(Freshness {
                    history,
                    declaration,
                    closure,
                })
            },
        )
    }
}

impl Comment {
    fn matches(&self, issue: &str, comment: &str) -> bool {
        if self.id != comment || self.issue.id != issue || !node(&self.issue.repository.id) {
            return false;
        }
        !self.created.is_empty() && !self.updated.is_empty()
    }
}

fn paired(value: &Value, pointer: &str, issue: &Subject, comment: &str) -> Option<Comment> {
    let found: Comment = field(value, pointer)?;
    (found.matches(&issue.id, comment) && found.issue.repository.id == issue.repository.id)
        .then_some(found)
}

fn history(target: &Target, window: &Window, value: &Value) -> Option<History> {
    target.issue(value)?;
    let issue = value.pointer("/data/repository/issue")?;
    let found = Subject::deserialize(issue).ok()?;
    let events: Page<Event> = page::page(issue, window)?;
    let repository = value.pointer("/data/repository/id")?.as_str()?;
    if !node(repository) || found.repository.id != repository || found.updated.is_empty() {
        return None;
    }
    let mut identities = BTreeSet::new();
    let valid = events
        .rows
        .iter()
        .all(|event| node(&event.id) && !event.created.is_empty() && identities.insert(&event.id));
    valid.then_some(History {
        issue: found,
        events,
    })
}

fn input(target: &Target, window: &Window) -> Value {
    let mut variables = target.variables();
    variables["size"] = json!(window.size);
    variables["after"] = json!(window.after);
    variables
}

fn query() -> String {
    format!(
        "repository(owner:$owner,name:$name) {{ id nameWithOwner issue(number:$number) {{ {HISTORY} {EVENTS} }} }}"
    )
}

fn edited<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(input)
}
