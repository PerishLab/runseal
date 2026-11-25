pub mod acceptance;
pub mod attached;
pub mod classify;
pub mod discussion;
pub mod phase;
pub mod records;
pub mod review;
pub mod search;
pub mod snapshot;

use super::{
    Call, Cause, Client, Error, Page, Receipt, Target, Window, field, graph, node, page, valid,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;

const ISSUE: &str = "query RunsealIssue($owner:String!,$name:String!,$number:Int!) { repository(owner:$owner,name:$name) { nameWithOwner issue(number:$number) { id number title body url state reason:stateReason updated:updatedAt writable:viewerCanUpdate kind:issueType { name } repository { id } } } }";
const KINDS: &str = "query RunsealKinds($owner:String!,$size:Int!,$after:String) { organization(login:$owner) { id login list:issueTypes(first:$size,after:$after) { total:totalCount rows:nodes { id name enabled:isEnabled } page:pageInfo { more:hasNextPage cursor:endCursor } } } }";
const CREATE: &str = "mutation RunsealCreateIssue($input:CreateIssueInput!) { createIssue(input:$input) { issue { id number title body url state reason:stateReason updated:updatedAt writable:viewerCanUpdate kind:issueType { id name } repository { id } } } }";
const COMMENT: &str = "mutation RunsealComment($issue: ID!, $body: String!) { addComment(input:{subjectId:$issue,body:$body}) { subject { id } commentEdge { node { id body url created:createdAt updated:updatedAt } } } }";
const CHILD: &str = "mutation RunsealChild($issue:ID!,$related:ID!) { link:addSubIssue(input:{issueId:$issue,subIssueId:$related,replaceParent:false}) { issue { id } related:subIssue { id } } }";
const BLOCKER: &str = "mutation RunsealBlocker($issue:ID!,$related:ID!) { link:addBlockedBy(input:{issueId:$issue,blockingIssueId:$related}) { issue { id } related:blockingIssue { id } } }";

#[derive(Clone, Debug)]
pub struct Draft {
    repository: String,
    title: String,
    body: String,
    kind: Option<String>,
}

impl Draft {
    pub fn new(
        repository: &str,
        title: &str,
        body: &str,
        kind: Option<&str>,
    ) -> Result<Self, Error> {
        if !node(repository) || kind.is_some_and(|id| !node(id)) {
            return Err(Error::local(Cause::Input));
        }
        if title.trim().is_empty() || title.chars().count() > 256 || body.len() > 65_536 {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            repository: repository.into(),
            title: title.into(),
            body: body.into(),
            kind: kind.map(str::to_owned),
        })
    }

    fn matches(&self, ticket: &Ticket, kind: Option<&str>) -> bool {
        if ticket.repository.id != self.repository || ticket.state != "OPEN" || ticket.number == 0 {
            return false;
        }
        if ticket.title != self.title || ticket.body != self.body || !node(&ticket.id) {
            return false;
        }
        self.kind
            .as_deref()
            .is_none_or(|expected| kind == Some(expected))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Ticket {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub body: String,
    pub url: String,
    pub state: String,
    #[serde(deserialize_with = "records::nullable")]
    pub reason: Option<String>,
    pub updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writable: Option<bool>,
    pub kind: Option<Label>,
    pub repository: Identity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Identity {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Kind {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    pub body: String,
    pub url: String,
    pub created: String,
    pub updated: String,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Relation {
    Child,
    Blocker,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Related {
    pub id: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Link {
    pub issue: Identity,
    pub related: Identity,
}

impl Client {
    pub fn create(&self, draft: &Draft) -> Result<Receipt<Ticket>, Error> {
        let mut input = json!({
            "repositoryId":draft.repository,
            "title":draft.title,
            "body":draft.body,
        });
        if let Some(kind) = &draft.kind {
            input["issueTypeId"] = json!(kind);
        }
        self.wire.send(
            &self.bearer,
            Call::Mutation(graph(CREATE, json!({"input":input}))),
            |value| {
                let issue = value.pointer("/data/createIssue/issue")?;
                let kind = issue
                    .pointer("/kind/id")
                    .and_then(serde_json::Value::as_str);
                let ticket = Ticket::deserialize(issue).ok()?;
                draft.matches(&ticket, kind).then_some(ticket)
            },
        )
    }

    pub fn issue(&self, target: &Target) -> Result<Receipt<Ticket>, Error> {
        self.wire.send(
            &self.bearer,
            Call::Query(graph(ISSUE, target.variables())),
            |value| {
                target.issue(&value)?;
                let found: Ticket = field(&value, "/data/repository/issue")?;
                (node(&found.id) && node(&found.repository.id)).then_some(found)
            },
        )
    }

    pub fn kinds(&self, owner: &str, window: &Window) -> Result<Receipt<Page<Kind>>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let input = json!({"owner":owner,"size":window.size,"after":window.after});
        self.wire
            .send(&self.bearer, Call::Query(graph(KINDS, input)), |value| {
                let scope = value.pointer("/data/organization")?;
                let login = scope.get("login")?.as_str()?;
                if !login.eq_ignore_ascii_case(owner) {
                    return None;
                }
                let found: Page<Kind> = page::page(scope, window)?;
                found
                    .rows
                    .iter()
                    .all(|kind| node(&kind.id) && !kind.name.trim().is_empty())
                    .then_some(found)
            })
    }

    pub fn comments(
        &self,
        target: &Target,
        window: &Window,
    ) -> Result<Receipt<Page<Comment>>, Error> {
        self.timeline(
            target,
            window,
            "comments",
            "id body url created:createdAt updated:updatedAt",
        )
    }

    pub fn labels(&self, target: &Target, window: &Window) -> Result<Receipt<Page<Label>>, Error> {
        self.timeline(target, window, "labels", "name")
    }

    pub fn relations(
        &self,
        target: &Target,
        window: &Window,
        relation: Relation,
    ) -> Result<Receipt<Page<Related>>, Error> {
        let connection = match relation {
            Relation::Child => "subIssues",
            Relation::Blocker => "blockedBy",
        };
        self.timeline(target, window, connection, "id state url")
    }

    fn timeline<T: DeserializeOwned>(
        &self,
        target: &Target,
        window: &Window,
        fields: &str,
        select: &str,
    ) -> Result<Receipt<Page<T>>, Error> {
        let query = format!(
            "query RunsealTimeline($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String) {{ repository(owner:$owner,name:$name) {{ nameWithOwner issue(number:$number) {{ id number list:{fields}(first:$size,after:$after) {{ total:totalCount rows:nodes {{ {select} }} page:pageInfo {{ more:hasNextPage cursor:endCursor }} }} }} }} }}"
        );
        let mut variables = target.variables();
        variables["size"] = json!(window.size);
        variables["after"] = json!(window.after);
        self.wire.send(
            &self.bearer,
            Call::Query(graph(&query, variables)),
            |value| {
                target.issue(&value)?;
                page::page(value.pointer("/data/repository/issue")?, window)
            },
        )
    }

    pub fn comment(&self, issue: &str, body: &str) -> Result<Receipt<Comment>, Error> {
        if !node(issue) || body.len() > 65_536 || body.trim().is_empty() {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"issue":issue,"body":body});
        self.wire.send(
            &self.bearer,
            Call::Mutation(graph(COMMENT, variables)),
            |value| {
                let subject: Identity = field(&value, "/data/addComment/subject")?;
                let found: Comment = field(&value, "/data/addComment/commentEdge/node")?;
                (subject.id == issue && found.body == body && node(&found.id)).then_some(found)
            },
        )
    }

    pub fn link(
        &self,
        issue: &str,
        related: &str,
        relation: Relation,
    ) -> Result<Receipt<Link>, Error> {
        if !node(issue) || !node(related) || issue == related {
            return Err(Error::local(Cause::Input));
        }
        let query = match relation {
            Relation::Child => CHILD,
            Relation::Blocker => BLOCKER,
        };
        let variables = json!({"issue":issue,"related":related});
        self.wire.send(
            &self.bearer,
            Call::Mutation(graph(query, variables)),
            |value| {
                let found: Link = field(&value, "/data/link")?;
                (found.issue.id == issue && found.related.id == related).then_some(found)
            },
        )
    }
}
