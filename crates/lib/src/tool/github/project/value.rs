use super::{
    Call, Cause, Client, Error, Identity, Overview, Page, Receipt, Window, activity, field, graph,
    node, page, structure, valid,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const ACCOUNT: &str = "query RunsealAccount($owner:String!) { organization(login:$owner) { id database:databaseId login url } }";
const SUBJECT: &str = "query RunsealSubject($id:ID!) { node(id:$id) { ... on Issue { id updated:updatedAt repository { id } } } }";
const CANDIDATES: &str = "query RunsealCandidates($ids:[ID!]!) { nodes(ids:$ids) { __typename ... on Issue { id number url state updatedAt issueType { id name } repository { id owner { id } } } } }";
const SEAT: &str = "query RunsealSeat($id:ID!) { node(id:$id) { __typename ... on Repository { id nameWithOwner owner { id } } } }";
const EXISTS: &str =
    "query RunsealExists($id:ID!) { node(id:$id) { __typename ... on Node { id } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    pub id: String,
    pub database: u64,
    pub login: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subject {
    pub id: String,
    pub updated: String,
    pub repository: Identity,
}

const VALUES: &str = "query RunsealValues($item:ID!,$size:Int!,$after:String) { node(id:$item) { ... on ProjectV2Item { id project { id } list:fieldValues(first:$size,after:$after) { total:totalCount rows:nodes {
    kind:__typename
    ... on ProjectV2ItemFieldTextValue { field { ... on ProjectV2FieldCommon { id } } text }
    ... on ProjectV2ItemFieldNumberValue { field { ... on ProjectV2FieldCommon { id } } number }
    ... on ProjectV2ItemFieldDateValue { field { ... on ProjectV2FieldCommon { id } } date }
    ... on ProjectV2ItemFieldSingleSelectValue { field { ... on ProjectV2FieldCommon { id } } option:optionId }
    ... on ProjectV2ItemFieldMultiSelectValue { field { ... on ProjectV2FieldCommon { id } } options { id } }
    ... on ProjectV2ItemFieldIterationValue { field { ... on ProjectV2FieldCommon { id } } iteration:iterationId }
} page:pageInfo { more:hasNextPage cursor:endCursor } } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub kind: String,
    pub field: Option<Identity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Identity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<String>,
}

impl Client {
    pub fn values(
        &self,
        project: &str,
        item: &str,
        window: &Window,
    ) -> Result<Receipt<Page<Entry>>, Error> {
        if !node(project) || !node(item) {
            return Err(Error::local(Cause::Input));
        }
        let input = json!({"item":item,"size":window.size,"after":window.after});
        self.wire
            .send(&self.bearer, Call::Query(graph(VALUES, input)), |value| {
                let found = value.pointer("/data/node")?;
                if found.pointer("/project/id")?.as_str()? != project {
                    return None;
                }
                let observed: Page<Entry> = page::page(found, window)?;
                (observed.target == item && observed.rows.iter().all(Entry::valid))
                    .then_some(observed)
            })
    }

    pub fn account(&self, owner: &str) -> Result<Receipt<Account>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.send(
            &self.bearer,
            Call::Query(graph(ACCOUNT, json!({"owner":owner}))),
            |value| {
                let found: Account = field(&value, "/data/organization")?;
                found.matches(owner).then_some(found)
            },
        )
    }

    pub fn metadata(&self, project: &str) -> Result<Receipt<Overview>, Error> {
        if !node(project) {
            return Err(Error::local(Cause::Input));
        }
        let statement = format!(
            "query RunsealMetadata($id:ID!) {{ node(id:$id) {{ ... on ProjectV2 {{ {} }} }} }}",
            structure::METADATA
        );
        self.wire.send(
            &self.bearer,
            Call::Query(graph(&statement, json!({"id":project}))),
            |value| {
                let found = Overview::deserialize(value.pointer("/data/node")?).ok()?;
                (found.id == project && found.valid()).then_some(found)
            },
        )
    }

    pub fn subject(&self, issue: &str) -> Result<Receipt<Subject>, Error> {
        if !node(issue) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.send(
            &self.bearer,
            Call::Query(graph(SUBJECT, json!({"id":issue}))),
            |value| {
                let found: Subject = field(&value, "/data/node")?;
                let valid =
                    found.id == issue && !found.updated.is_empty() && node(&found.repository.id);
                valid.then_some(found)
            },
        )
    }

    pub fn candidates(&self, ids: &[String]) -> Result<Receipt<Vec<Value>>, Error> {
        if ids.is_empty() || ids.len() > 50 || !ids.iter().all(|id| node(id)) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.send(
            &self.bearer,
            Call::Query(graph(CANDIDATES, json!({"ids":ids}))),
            |value| {
                let nodes = value.pointer("/data/nodes")?.as_array()?;
                (nodes.len() == ids.len()).then(|| nodes.clone())
            },
        )
    }

    pub fn seat(&self, repository: &str) -> Result<Receipt<Value>, Error> {
        if !node(repository) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.send(
            &self.bearer,
            Call::Query(graph(SEAT, json!({"id":repository}))),
            |value| value.pointer("/data/node").cloned(),
        )
    }

    pub fn exists(&self, id: &str) -> Result<Receipt<Value>, Error> {
        if !node(id) {
            return Err(Error::local(Cause::Input));
        }
        self.wire.send(
            &self.bearer,
            Call::Query(graph(EXISTS, json!({"id":id}))),
            |value| value.pointer("/data/node").cloned(),
        )
    }

    pub fn confirm(&self, issue: &str) -> Result<Receipt<Value>, Error> {
        if !node(issue) {
            return Err(Error::local(Cause::Input));
        }
        let statement = format!(
            "query RunsealConfirm($id:ID!) {{ node(id:$id) {{ ... on Issue {{ {} }} }} }}",
            activity::NEXT
        );
        self.wire.send(
            &self.bearer,
            Call::Query(graph(&statement, json!({"id":issue}))),
            |value| value.pointer("/data/node").cloned(),
        )
    }
}

impl Account {
    fn matches(&self, owner: &str) -> bool {
        if self.database == 0 || !self.login.eq_ignore_ascii_case(owner) {
            return false;
        }
        let expected = format!("https://github.com/{}", self.login);
        node(&self.id) && self.url.eq_ignore_ascii_case(&expected)
    }
}

impl Entry {
    fn valid(&self) -> bool {
        match self.kind.as_str() {
            "ProjectV2ItemFieldTextValue"
            | "ProjectV2ItemFieldNumberValue"
            | "ProjectV2ItemFieldDateValue"
            | "ProjectV2ItemFieldSingleSelectValue"
            | "ProjectV2ItemFieldMultiSelectValue"
            | "ProjectV2ItemFieldIterationValue" => {
                self.field.as_ref().is_some_and(|field| node(&field.id))
            }
            _ => !self.kind.is_empty(),
        }
    }
}
