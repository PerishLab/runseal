use super::{Call, Cause, Client, Error, Identity, Page, Receipt, Window, graph, node, page};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
