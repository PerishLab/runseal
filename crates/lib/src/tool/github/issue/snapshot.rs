use super::{Call, Client, Error, Receipt, Target, Window, field, graph, node};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const QUERY: &str = r#"
query RunsealSnapshot(
  $owner: String!, $name: String!, $number: Int!, $first: Int!,
  $subAfter: String, $blockedAfter: String, $blockingAfter: String,
  $pullsAfter: String, $commentsAfter: String
) {
  repository(owner: $owner, name: $name) {
    id nameWithOwner
    issue(number: $number) {
      id number url title body state updatedAt
      issueType { name }
      repository { nameWithOwner }
      parent { id number url title state repository { nameWithOwner } }
      subIssues(first: $first, after: $subAfter) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes { id number url title state repository { nameWithOwner } }
      }
      subIssuesSummary { total completed percentCompleted }
      blockedBy(first: $first, after: $blockedAfter) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes { id number url title state repository { nameWithOwner } }
      }
      blocking(first: $first, after: $blockingAfter) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes { id number url title state repository { nameWithOwner } }
      }
      timelineItems(first: $first, after: $pullsAfter, itemTypes: [CROSS_REFERENCED_EVENT]) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes {
          ... on CrossReferencedEvent {
            source {
              __typename
              ... on PullRequest {
                id number url title state mergedAt updatedAt
                repository { nameWithOwner }
              }
            }
          }
        }
      }
      comments(first: $first, after: $commentsAfter) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes { body url }
      }
    }
  }
}
"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub number: i64,
    pub url: String,
    pub title: String,
    pub body: String,
    pub state: String,
    #[serde(rename = "updatedAt")]
    pub updated: String,
    #[serde(rename = "issueType")]
    pub kind: Option<Type>,
    pub repository: Name,
    pub parent: Option<Related>,
    #[serde(rename = "subIssues")]
    pub children: Connection<Related>,
    #[serde(rename = "subIssuesSummary")]
    pub summary: Summary,
    #[serde(rename = "blockedBy")]
    pub blockers: Connection<Related>,
    pub blocking: Connection<Related>,
    #[serde(rename = "timelineItems")]
    pub references: Connection<Event>,
    pub comments: Connection<Comment>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Type {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Related {
    pub id: String,
    pub number: i64,
    pub url: String,
    pub title: String,
    pub state: String,
    pub repository: Name,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    #[serde(rename = "nameWithOwner")]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection<T> {
    #[serde(rename = "totalCount")]
    pub total: usize,
    #[serde(rename = "pageInfo")]
    pub position: Position,
    pub nodes: Vec<T>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    #[serde(rename = "hasNextPage")]
    pub more: bool,
    #[serde(rename = "endCursor")]
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub total: usize,
    pub completed: usize,
    #[serde(rename = "percentCompleted")]
    pub percent: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub source: Option<Source>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(rename = "__typename")]
    pub kind: String,
    pub id: Option<String>,
    pub number: Option<i64>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "mergedAt")]
    pub merged: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated: Option<String>,
    pub repository: Option<Name>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Comment {
    pub body: String,
    pub url: String,
}

#[derive(Debug, Default, clap::Args)]
pub struct Cursors {
    #[arg(long)]
    pub children: Option<String>,
    #[arg(long)]
    pub blockers: Option<String>,
    #[arg(long)]
    pub blocking: Option<String>,
    #[arg(long)]
    pub pulls: Option<String>,
    #[arg(long)]
    pub comments: Option<String>,
}

impl Client {
    pub fn snapshot(
        &self,
        target: &Target,
        size: u8,
        cursors: &Cursors,
    ) -> Result<Receipt<Issue>, Error> {
        let variables = cursors.variables(target, size)?;
        self.wire.send(
            &self.bearer,
            Call::Query(graph(QUERY, variables)),
            |value| {
                target.issue(&value)?;
                let repo = value.pointer("/data/repository")?;
                if !node(repo.get("id")?.as_str()?) {
                    return None;
                }
                let issue: Issue = field(repo, "/issue")?;
                issue.matches(target, size, cursors).then_some(issue)
            },
        )
    }
}

impl Cursors {
    fn variables(&self, target: &Target, size: u8) -> Result<Value, Error> {
        let mut variables = target.variables();
        variables["first"] = size.into();
        for (field, value) in [
            ("subAfter", &self.children),
            ("blockedAfter", &self.blockers),
            ("blockingAfter", &self.blocking),
            ("pullsAfter", &self.pulls),
            ("commentsAfter", &self.comments),
        ] {
            Window::new(size, value.as_deref())?;
            if let Some(cursor) = value {
                variables[field] = json!(cursor);
            }
        }
        Ok(variables)
    }
}

impl Issue {
    fn matches(&self, target: &Target, size: u8, cursors: &Cursors) -> bool {
        let url = format!(
            "https://github.com/{}/issues/{}",
            target.name(),
            target.number()
        );
        let identity = node(&self.id)
            && self.repository.name.eq_ignore_ascii_case(&target.name())
            && self.url.eq_ignore_ascii_case(&url);
        identity && self.relations(size, cursors) && self.timeline(size, cursors)
    }
    fn relations(&self, size: u8, cursors: &Cursors) -> bool {
        self.children.matches(size, &cursors.children)
            && self.blockers.matches(size, &cursors.blockers)
            && self.blocking.matches(size, &cursors.blocking)
    }

    fn timeline(&self, size: u8, cursors: &Cursors) -> bool {
        self.references.matches(size, &cursors.pulls)
            && self.comments.matches(size, &cursors.comments)
    }
}

impl<T> Connection<T> {
    fn matches(&self, size: u8, after: &Option<String>) -> bool {
        if self.nodes.len() > usize::from(size) || self.nodes.len() > self.total {
            return false;
        }
        if !self.position.more {
            return true;
        }
        let Some(next) = &self.position.cursor else {
            return false;
        };
        !self.nodes.is_empty()
            && after.as_ref() != Some(next)
            && Window::new(size, Some(next)).is_ok()
    }
}
