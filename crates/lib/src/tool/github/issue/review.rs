use super::{Call, Client, Error, Label, Page, Receipt, Target, Ticket, Window, graph, node, page};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const QUERY: &str = r#"
query RunsealAcceptancePage($owner:String!,$name:String!,$number:Int!,$labelsSize:Int!,$labelsAfter:String,$commentsSize:Int!,$commentsAfter:String) {
  repository(owner:$owner,name:$name) {
    id nameWithOwner
    issue(number:$number) {
      id number title body url state reason:stateReason updated:updatedAt writable:viewerCanUpdate
      kind:issueType { name }
      repository { id }
      labels(first:$labelsSize,after:$labelsAfter) {
        total:totalCount rows:nodes { name } page:pageInfo { more:hasNextPage cursor:endCursor }
      }
      comments(first:$commentsSize,after:$commentsAfter) {
        total:totalCount rows:nodes { id body url } page:pageInfo { more:hasNextPage cursor:endCursor }
      }
    }
  }
}
"#;

const RELATIONS: &str = r#"
query RunsealAcceptanceRelations($owner:String!,$name:String!,$number:Int!,$childrenSize:Int!,$childrenAfter:String,$blockersSize:Int!,$blockersAfter:String) {
  repository(owner:$owner,name:$name) {
    id nameWithOwner
    issue(number:$number) {
      id number title body url state reason:stateReason updated:updatedAt writable:viewerCanUpdate
      kind:issueType { name }
      repository { id }
      children:subIssues(first:$childrenSize,after:$childrenAfter) {
        total:totalCount rows:nodes { id state } page:pageInfo { more:hasNextPage cursor:endCursor }
      }
      blockers:blockedBy(first:$blockersSize,after:$blockersAfter) {
        total:totalCount rows:nodes { id state } page:pageInfo { more:hasNextPage cursor:endCursor }
      }
    }
  }
}
"#;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub id: String,
    pub body: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct Review {
    pub issue: Ticket,
    pub labels: Page<Label>,
    pub comments: Page<Entry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    pub id: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct Relations {
    pub issue: Ticket,
    pub children: Page<Status>,
    pub blockers: Page<Status>,
}

impl Client {
    pub fn acceptance(
        &self,
        target: &Target,
        labels: &Window,
        comments: &Window,
    ) -> Result<Receipt<Review>, Error> {
        let mut variables = target.variables();
        variables["labelsSize"] = json!(labels.size);
        variables["labelsAfter"] = json!(labels.after);
        variables["commentsSize"] = json!(comments.size);
        variables["commentsAfter"] = json!(comments.after);
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(QUERY, variables)),
            |value| observed(target, &value, labels, comments),
        )
    }

    pub fn dependencies(
        &self,
        target: &Target,
        children: &Window,
        blockers: &Window,
    ) -> Result<Receipt<Relations>, Error> {
        let mut variables = target.variables();
        variables["childrenSize"] = json!(children.size);
        variables["childrenAfter"] = json!(children.after);
        variables["blockersSize"] = json!(blockers.size);
        variables["blockersAfter"] = json!(blockers.after);
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(RELATIONS, variables)),
            |value| relations(target, &value, children, blockers),
        )
    }
}

fn observed(target: &Target, value: &Value, labels: &Window, comments: &Window) -> Option<Review> {
    let (issue, scope) = subject(target, value)?;
    let labels: Page<Label> = page::connection(scope, "labels", labels)?;
    let comments: Page<Entry> = page::connection(scope, "comments", comments)?;
    let prefix = format!("{}#issuecomment-", issue.url);
    if labels.rows.iter().any(|label| label.name.trim().is_empty())
        || comments.rows.iter().any(|entry| !entry.matches(&prefix))
    {
        return None;
    }
    Some(Review {
        issue,
        labels,
        comments,
    })
}

fn relations(
    target: &Target,
    value: &Value,
    children: &Window,
    blockers: &Window,
) -> Option<Relations> {
    let (issue, scope) = subject(target, value)?;
    let children: Page<Status> = page::connection(scope, "children", children)?;
    let blockers: Page<Status> = page::connection(scope, "blockers", blockers)?;
    let valid = children
        .rows
        .iter()
        .chain(&blockers.rows)
        .all(Status::valid);
    valid.then_some(Relations {
        issue,
        children,
        blockers,
    })
}

fn subject<'a>(target: &Target, value: &'a Value) -> Option<(Ticket, &'a Value)> {
    target.issue(value)?;
    let repository = value.pointer("/data/repository")?;
    let name = repository.get("nameWithOwner")?.as_str()?;
    let scope = repository.get("issue")?;
    let issue = Ticket::deserialize(scope).ok()?;
    let url = format!("https://github.com/{name}/issues/{}", target.number());
    let identity = [
        node(&issue.id),
        node(&issue.repository.id),
        repository.get("id")?.as_str()? == issue.repository.id,
        issue.url == url,
        !issue.updated.is_empty(),
        issue.writable.is_some(),
        matches!(issue.state.as_str(), "OPEN" | "CLOSED"),
    ];
    if !identity.into_iter().all(|valid| valid) {
        return None;
    }
    Some((issue, scope))
}

impl Status {
    fn valid(&self) -> bool {
        node(&self.id) && matches!(self.state.as_str(), "OPEN" | "CLOSED")
    }
}

impl Entry {
    fn matches(&self, prefix: &str) -> bool {
        let number = self
            .url
            .strip_prefix(prefix)
            .and_then(|value| value.parse::<u64>().ok());
        node(&self.id) && number.is_some_and(|number| number > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::tests::{reply, serve};
    use crate::github::{Cause, Outcome};

    const FIXTURE: &str = r#"{"data":{"repository":{
        "id":"R_fixture","nameWithOwner":"PerishLab/runseal",
        "issue":{
            "id":"I_fixture","number":229,"title":"Issue","body":"Body",
            "url":"https://github.com/PerishLab/runseal/issues/229","state":"OPEN",
            "reason":null,"writable":true,"updated":"2026-11-01T00:00:00Z",
            "kind":{"name":"Feature"},"repository":{"id":"R_fixture"},
            "labels":{"total":4,"rows":[{"name":"retained"}],"page":{"more":true,"cursor":"cursor-label-next"}},
            "comments":{"total":2,"rows":[{"id":"IC_fixture","body":"Review","url":"https://github.com/PerishLab/runseal/issues/229#issuecomment-4"}],"page":{"more":false,"cursor":"cursor-comment-last"}}
        }
    }}}"#;

    #[test]
    fn connections() {
        let server = serve(vec![reply("200 OK", "", FIXTURE)]);
        let target = Target::new("PerishLab", "runseal", 229).unwrap();
        let labels = Window::new(1, Some("cursor-label")).unwrap();
        let comments = Window::new(2, Some("cursor-comment")).unwrap();
        let receipt = server
            .client
            .acceptance(&target, &labels, &comments)
            .unwrap();
        let value = receipt.value;
        assert_eq!(receipt.outcome, Outcome::Observed);
        assert_eq!(value.issue.repository.id, "R_fixture");
        assert_eq!(value.labels.target, value.comments.target);
        assert_eq!(value.labels.next.as_deref(), Some("cursor-label-next"));
        assert_eq!(value.comments.total, 2);
        assert_eq!(value.comments.rows[0].id, "IC_fixture");
        assert!(value.comments.next.is_none());
        let requests = server.thread.join().unwrap();
        let input: Value =
            serde_json::from_str(requests[0].rsplit_once("\r\n").unwrap().1).unwrap();
        assert_eq!(input["variables"]["labelsAfter"], "cursor-label");
        assert_eq!(input["variables"]["commentsAfter"], "cursor-comment");
    }

    #[test]
    fn dependencies() {
        let mut data: Value = serde_json::from_str(FIXTURE).unwrap();
        let issue = &mut data["data"]["repository"]["issue"];
        issue["children"] = json!({"total":1,"rows":[{"id":"I_child","state":"OPEN"}],"page":{"more":false,"cursor":null}});
        issue["blockers"] = json!({"total":1,"rows":[{"id":"I_blocker","state":"CLOSED"}],"page":{"more":false,"cursor":null}});
        let server = serve(vec![reply("200 OK", "", &data.to_string())]);
        let target = Target::new("PerishLab", "runseal", 229).unwrap();
        let window = Window::new(100, None).unwrap();
        let value = server
            .client
            .dependencies(&target, &window, &window)
            .unwrap()
            .value;
        assert_eq!(value.issue.id, value.children.target);
        assert_eq!(value.issue.id, value.blockers.target);
        assert_eq!(value.children.rows[0].state, "OPEN");
        assert_eq!(value.blockers.rows[0].state, "CLOSED");
        assert!(server.thread.join().unwrap()[0].contains("query RunsealAcceptanceRelations"));
    }

    #[test]
    fn missing() {
        let mut bad: Value = serde_json::from_str(FIXTURE).unwrap();
        bad["data"]["repository"]["issue"]
            .as_object_mut()
            .unwrap()
            .remove("reason");
        let target = Target::new("PerishLab", "runseal", 229).unwrap();
        let window = Window::new(100, None).unwrap();
        let server = serve(vec![reply("200 OK", "", &bad.to_string())]);
        let error = server
            .client
            .acceptance(&target, &window, &window)
            .unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
        server.thread.join().unwrap();
        let server = serve(vec![reply("200 OK", "", &bad.to_string())]);
        let error = server.client.issue(&target).unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
        server.thread.join().unwrap();
    }
}
