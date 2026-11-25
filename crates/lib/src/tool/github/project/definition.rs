use super::{
    Call, Cause, Client, Error, Identity, Page, Receipt, Window, activity, connection, graph, node,
    page, selection, valid,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const CATALOG: &str = "query RunsealCatalog($owner:String!,$size:Int!,$after:String){organization(login:$owner){id login url repositories(first:$size,after:$after){totalCount pageInfo{hasNextPage endCursor} nodes{id nameWithOwner url isArchived hasIssuesEnabled owner{id login}}}}}";
const OPEN: &str = "query RunsealOpen($id:ID!,$size:Int!,$after:String){node(id:$id){... on Repository{id nameWithOwner url owner{id login} issues(first:$size,states:OPEN,after:$after){totalCount pageInfo{hasNextPage endCursor} nodes{id number title url state updatedAt issueType{id name} repository{id nameWithOwner owner{id login}}}}}}}";
const ORIGINS: &str = "query RunsealOrigins($owner:String!,$size:Int!,$after:String){organization(login:$owner){id login issueFields(first:$size,after:$after){totalCount pageInfo{hasNextPage endCursor} nodes{__typename ... on IssueFieldSingleSelect{id fullDatabaseId name dataType visibility options{id name color description}} ... on IssueFieldText{id} ... on IssueFieldNumber{id} ... on IssueFieldDate{id} ... on IssueFieldMultiSelect{id}}}}}";

#[derive(Debug, Serialize)]
pub struct Catalog {
    pub owner: Value,
    pub repositories: Value,
}

#[derive(Debug, Serialize)]
pub struct Open {
    pub repository: Value,
    pub issues: Value,
}

#[derive(Debug, Serialize)]
pub struct Origins {
    pub owner: String,
    pub fields: Value,
}

const FIELDS: &str = "query RunsealFields($owner:String!,$number:Int!,$size:Int!,$after:String) { organization(login:$owner) { login projectV2(number:$number) { id number list:fields(first:$size,after:$after) { total:totalCount rows:nodes {
    kind:__typename
    ... on ProjectV2FieldCommon { id name type:dataType issue:isIssueField project { id } }
    ... on ProjectV2Field { origin:issueField { ... on Node { id } } }
    ... on ProjectV2SingleSelectField { origin:issueField { ... on Node { id } } options { id name color description } }
    ... on ProjectV2MultiSelectField { origin:issueField { ... on Node { id } } multiple:multiSelectOptions { id name color description } }
    ... on ProjectV2IterationField { configuration { duration start:startDay current:iterations { id title start:startDate duration } past:completedIterations { id title start:startDate duration } } }
} page:pageInfo { more:hasNextPage cursor:endCursor } } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(rename = "type")]
    pub datatype: String,
    pub issue: bool,
    pub project: Identity,
    pub origin: Option<Identity>,
    pub options: Option<Vec<Choice>>,
    pub multiple: Option<Vec<Choice>>,
    pub configuration: Option<Schedule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Choice {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schedule {
    pub start: u8,
    pub duration: u16,
    pub current: Vec<Iteration>,
    pub past: Vec<Iteration>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Iteration {
    pub id: String,
    pub title: String,
    pub start: String,
    pub duration: u16,
}

impl Client {
    pub fn fields(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<Definition>>, Error> {
        let input = selection(owner, number, window)?;
        self.wire
            .send(&self.bearer, Call::Query(graph(FIELDS, input)), |value| {
                let scope = connection(&value, owner, number)?;
                let observed: Page<Definition> = page::page(scope, window)?;
                observed
                    .rows
                    .iter()
                    .all(|field| field.matches(&observed.target))
                    .then_some(observed)
            })
    }

    pub fn catalog(&self, owner: &str, window: &Window) -> Result<Receipt<Catalog>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(CATALOG, variables)),
            |value| catalog(value, owner, window),
        )
    }

    pub fn backlog(&self, repository: &str, window: &Window) -> Result<Receipt<Open>, Error> {
        if !node(repository) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"id":repository,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(OPEN, variables)),
            |value| open(value, repository, window),
        )
    }

    pub fn origins(&self, owner: &str, window: &Window) -> Result<Receipt<Origins>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Query(graph(ORIGINS, variables)),
            |value| origins(value, owner, window),
        )
    }
}

fn catalog(value: Value, expected: &str, window: &Window) -> Option<Catalog> {
    let mut owner = value.pointer("/data/organization")?.as_object()?.clone();
    let login = owner.get("login")?.as_str()?;
    let id = owner.get("id")?.as_str()?;
    if !node(id) || !login.eq_ignore_ascii_case(expected) {
        return None;
    }
    let repositories = owner.remove("repositories")?;
    activity::checked(&repositories, window)?;
    Some(Catalog {
        owner: Value::Object(owner),
        repositories,
    })
}

fn open(value: Value, expected: &str, window: &Window) -> Option<Open> {
    let mut repository = value.pointer("/data/node")?.as_object()?.clone();
    if repository.get("id").and_then(Value::as_str) != Some(expected) {
        return None;
    }
    let issues = repository.remove("issues")?;
    activity::checked(&issues, window)?;
    Some(Open {
        repository: Value::Object(repository),
        issues,
    })
}

fn origins(value: Value, expected: &str, window: &Window) -> Option<Origins> {
    let owner = value.pointer("/data/organization")?;
    let id = owner.get("id")?.as_str()?;
    let login = owner.get("login")?.as_str()?;
    if !node(id) || !login.eq_ignore_ascii_case(expected) {
        return None;
    }
    let fields = owner.get("issueFields")?.clone();
    activity::checked(&fields, window)?;
    Some(Origins {
        owner: id.into(),
        fields,
    })
}

impl Definition {
    fn matches(&self, project: &str) -> bool {
        if !node(&self.id) || self.project.id != project || self.name.trim().is_empty() {
            return false;
        }
        self.options
            .iter()
            .chain(&self.multiple)
            .flatten()
            .all(|option| node(&option.id))
    }
}
