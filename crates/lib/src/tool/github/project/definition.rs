use super::{
    Call, Client, Error, Identity, Page, Receipt, Window, connection, graph, node, page, selection,
};
use serde::{Deserialize, Serialize};

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
