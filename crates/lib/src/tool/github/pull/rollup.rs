use super::{Call, Client, Error, Receipt, Target, sha};
use crate::github::{Cause, Page, Window, graph, node, page};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const QUERY: &str = r#"
query RunsealRollup($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String) {
  repository(owner:$owner,name:$name) {
    id nameWithOwner
    pullRequest(number:$number) {
      id number head:headRefOid
      commits(last:1) {
        nodes {
          commit {
            oid
            statusCheckRollup {
              id
              list:contexts(first:$size,after:$after) {
                total:totalCount
                rows:nodes {
                  __typename
                  ... on CheckRun { id name status conclusion detailsUrl }
                  ... on StatusContext { id context state targetUrl }
                }
                page:pageInfo { more:hasNextPage cursor:endCursor }
              }
            }
          }
        }
      }
    }
  }
}"#;

#[derive(Debug, Deserialize, Serialize)]
pub struct Rollup {
    pub pull: String,
    pub repository: String,
    pub head: String,
    pub contexts: Option<Page<Context>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "__typename")]
pub enum Context {
    #[serde(rename = "CheckRun")]
    Run {
        id: String,
        name: String,
        status: String,
        conclusion: Option<String>,
        #[serde(rename = "detailsUrl")]
        url: Option<String>,
    },
    #[serde(rename = "StatusContext")]
    Status {
        id: String,
        context: String,
        state: String,
        #[serde(rename = "targetUrl")]
        url: Option<String>,
    },
}

impl Client {
    pub fn rollup(
        &self,
        target: &Target,
        expected: &str,
        window: &Window,
    ) -> Result<Receipt<Rollup>, Error> {
        if !sha(expected) {
            return Err(Error::local(Cause::Input));
        }
        let mut variables = target.variables();
        variables["size"] = window.size.into();
        variables["after"] = serde_json::json!(window.after);
        self.wire.send(
            &self.bearer,
            Call::Query(graph(QUERY, variables)),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let name = repo.get("nameWithOwner")?.as_str()?;
                let repository = repo.get("id")?.as_str()?;
                if !name.eq_ignore_ascii_case(&target.name()) || !node(repository) {
                    return None;
                }
                let pull = repo.get("pullRequest")?;
                let id = pull.get("id")?.as_str()?;
                let head = pull.get("head")?.as_str()?;
                if !node(id)
                    || head != expected
                    || pull.get("number")?.as_u64()? != u64::from(target.number())
                {
                    return None;
                }
                let [revision] = pull.pointer("/commits/nodes")?.as_array()?.as_slice() else {
                    return None;
                };
                let commit = revision.get("commit")?;
                if commit.get("oid")?.as_str()? != head {
                    return None;
                }
                let contexts = contexts(commit, window)?;
                Some(Rollup {
                    pull: id.into(),
                    repository: repository.into(),
                    head: head.into(),
                    contexts,
                })
            },
        )
    }
}

fn contexts(commit: &serde_json::Value, window: &Window) -> Option<Option<Page<Context>>> {
    let rollup = commit.get("statusCheckRollup")?;
    if rollup.is_null() {
        return window.after.is_none().then_some(None);
    }
    let contexts: Page<Context> = page::page(rollup, window)?;
    let mut seen = BTreeSet::new();
    contexts
        .rows
        .iter()
        .all(|row| row.valid() && seen.insert(row.id()))
        .then_some(Some(contexts))
}

impl Context {
    pub fn id(&self) -> &str {
        match self {
            Self::Run { id, .. } | Self::Status { id, .. } => id,
        }
    }

    fn valid(&self) -> bool {
        let (name, state) = match self {
            Self::Run { name, status, .. } => (name, status),
            Self::Status { context, state, .. } => (context, state),
        };
        node(self.id()) && !name.trim().is_empty() && !state.is_empty()
    }
}
