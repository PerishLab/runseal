use super::{Call, Cause, Client, Error, Identity, Receipt, date, graph, mutation, node};
use crate::github::{Outcome, field};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const FIELD: &str = "field:projectV2Field {
    ... on ProjectV2FieldCommon { id name issue:isIssueField project { id } }
    ... on ProjectV2Field { origin:issueField { ... on Node { id } } }
    ... on ProjectV2SingleSelectField { origin:issueField { ... on Node { id } } }
    ... on ProjectV2MultiSelectField { origin:issueField { ... on Node { id } } }
}";
const DELETED: &str = "field:projectV2Field { ... on ProjectV2FieldCommon { id } }";

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Schema {
    Create {
        project: String,
        name: String,
        definition: Shape,
    },
    Update {
        field: String,
        name: Option<String>,
        definition: Option<Shape>,
    },
    Attach {
        project: String,
        origin: String,
    },
    Remove {
        field: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Shape {
    Text,
    Number,
    Date,
    Single(Vec<Alternative>),
    Multiple(Vec<Alternative>),
    Iteration(Calendar),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub color: Color,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Color {
    Gray,
    Blue,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Calendar {
    #[serde(rename = "startDate")]
    pub start: String,
    pub duration: i32,
    pub iterations: Vec<Period>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "startDate")]
    pub start: String,
    pub duration: i32,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Descriptor {
    pub id: String,
    pub name: Option<String>,
    pub issue: Option<bool>,
    pub project: Option<Identity>,
    pub origin: Option<Identity>,
}

impl Client {
    pub fn schema(&self, schema: &Schema) -> Result<Receipt<Descriptor>, Error> {
        let (query, input) = schema.request().ok_or_else(|| Error::local(Cause::Input))?;
        let call = Call::Mutation(graph(&query, json!({"input":input})));
        let mut receipt = self.wire.send(&self.bearer, call, |value| {
            let field = field(&value, "/data/change/field")?;
            schema.matches(&field).then_some(field)
        })?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Schema {
    fn request(&self) -> Option<(String, Value)> {
        match self {
            Self::Create {
                project,
                name,
                definition,
            } if node(project) && named(name) => {
                let mut input =
                    json!({"projectId": project, "name": name, "dataType": definition.kind()});
                if definition.configurable() {
                    let (key, value) = definition.configuration()?;
                    input[key] = value;
                }
                Some((
                    mutation("createProjectV2Field", "CreateProjectV2FieldInput", FIELD),
                    input,
                ))
            }
            Self::Update {
                field,
                name,
                definition,
            } if node(field) => {
                let input = update(field, name.as_deref(), definition.as_ref())?;
                Some((
                    mutation("updateProjectV2Field", "UpdateProjectV2FieldInput", FIELD),
                    input,
                ))
            }
            Self::Attach { project, origin } if node(project) && node(origin) => Some((
                mutation(
                    "createProjectV2IssueField",
                    "CreateProjectV2IssueFieldInput",
                    FIELD,
                ),
                json!({"projectId": project, "issueFieldId": origin}),
            )),
            Self::Remove { field } if node(field) => Some((
                mutation("deleteProjectV2Field", "DeleteProjectV2FieldInput", DELETED),
                json!({"fieldId": field}),
            )),
            _ => None,
        }
    }

    fn matches(&self, found: &Descriptor) -> bool {
        if !node(&found.id) {
            return false;
        }
        let project = found.project.as_ref().map(|identity| identity.id.as_str());
        let actual = found.name.as_deref();
        match self {
            Self::Create {
                project: expected,
                name,
                ..
            } => project == Some(expected.as_str()) && actual == Some(name.as_str()),
            Self::Update { field, name, .. } => {
                found.id == *field && name.as_deref().is_none_or(|name| actual == Some(name))
            }
            Self::Attach {
                project: expected,
                origin,
            } => {
                let actual = found.origin.as_ref().map(|identity| identity.id.as_str());
                project == Some(expected.as_str())
                    && found.issue == Some(true)
                    && actual == Some(origin.as_str())
            }
            Self::Remove { field } => found.id == *field,
        }
    }
}

impl Shape {
    fn kind(&self) -> &str {
        match self {
            Self::Text => "TEXT",
            Self::Number => "NUMBER",
            Self::Date => "DATE",
            Self::Single(_) => "SINGLE_SELECT",
            Self::Multiple(_) => "MULTI_SELECT",
            Self::Iteration(_) => "ITERATION",
        }
    }

    fn configurable(&self) -> bool {
        matches!(
            self,
            Self::Single(_) | Self::Multiple(_) | Self::Iteration(_)
        )
    }

    fn configuration(&self) -> Option<(&str, Value)> {
        match self {
            Self::Single(options) if alternatives(options) => {
                Some(("singleSelectOptions", json!(options)))
            }
            Self::Multiple(options) if alternatives(options) => {
                Some(("multiSelectOptions", json!(options)))
            }
            Self::Iteration(calendar) if calendar.valid() => {
                Some(("iterationConfiguration", json!(calendar)))
            }
            _ => None,
        }
    }
}

impl Calendar {
    fn valid(&self) -> bool {
        if self.duration <= 0 || !date(&self.start) {
            return false;
        }
        let mut seen = BTreeSet::new();
        self.iterations
            .iter()
            .all(|period| period.valid() && period.id.as_ref().is_none_or(|id| seen.insert(id)))
    }
}

impl Period {
    fn valid(&self) -> bool {
        if self.duration <= 0 || !date(&self.start) || !named(&self.title) {
            return false;
        }
        self.id.as_deref().is_none_or(node)
    }
}

fn alternatives(options: &[Alternative]) -> bool {
    if options.is_empty() {
        return false;
    }
    let mut seen = BTreeSet::new();
    options.iter().all(|option| {
        named(&option.name)
            && option
                .id
                .as_ref()
                .is_none_or(|id| node(id) && seen.insert(id))
    })
}

fn update(field: &str, name: Option<&str>, definition: Option<&Shape>) -> Option<Value> {
    if name.is_none() && definition.is_none() {
        return None;
    }
    let mut input = json!({"fieldId":field});
    if let Some(name) = name {
        if !named(name) {
            return None;
        }
        input["name"] = json!(name);
    }
    if let Some(definition) = definition {
        let (key, value) = definition.configuration()?;
        input[key] = value;
    }
    Some(input)
}

fn named(value: &str) -> bool {
    !value.trim().is_empty()
}
