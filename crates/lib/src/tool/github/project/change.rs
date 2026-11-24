use super::{Call, Cause, Client, Error, Receipt, date, graph, node};
use crate::github::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub project: String,
    pub action: Action,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Action {
    Add {
        content: String,
    },
    Remove {
        item: String,
    },
    Move {
        item: String,
        after: Option<String>,
    },
    Set {
        item: String,
        field: String,
        value: Field,
    },
    Clear {
        item: String,
        field: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    Text(String),
    Number(f64),
    Date(String),
    #[serde(rename = "singleSelectOptionId")]
    Select(String),
    #[serde(rename = "multiSelectOptionIds")]
    Multiple(Vec<String>),
    #[serde(rename = "iterationId")]
    Iteration(String),
}

#[derive(Debug, Serialize)]
pub struct Acknowledgement {
    pub project: String,
    pub item: String,
    pub content: Option<String>,
}

struct Mutation {
    name: &'static str,
    kind: &'static str,
    output: &'static str,
}

impl Mutation {
    fn new(name: &'static str, kind: &'static str, output: &'static str) -> Self {
        Self { name, kind, output }
    }
}

impl Client {
    pub fn change(&self, change: &Change) -> Result<Receipt<Acknowledgement>, Error> {
        let (query, input) = change.request().ok_or_else(|| Error::local(Cause::Input))?;
        let sent = Call::Mutation(graph(&query, json!({"input":input})));
        let mut receipt = self.wire.send(&self.bearer, sent, |value| {
            change.acknowledgement(value.pointer("/data/change")?)
        })?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Change {
    fn request(&self) -> Option<(String, Value)> {
        if !node(&self.project) || !self.action.valid() {
            return None;
        }
        let (mutation, mut input) = self.action.request();
        input["projectId"] = json!(self.project);
        let query = format!(
            "mutation RunsealChange($input:{}!) {{ change:{}(input:$input) {{ {} }} }}",
            mutation.kind, mutation.name, mutation.output
        );
        Some((query, input))
    }

    fn acknowledgement(&self, value: &Value) -> Option<Acknowledgement> {
        let (item, content) = match &self.action {
            Action::Add { content } => {
                let item = self.item(value)?;
                let actual = value.pointer("/item/content/id")?.as_str()?;
                (actual == content).then_some((item, Some(actual)))?
            }
            Action::Remove { item } => {
                let actual = value.get("deletedItemId")?.as_str()?;
                (actual == item).then_some((actual, None))?
            }
            Action::Move { item, .. } => {
                value.pointer("/items/totalCount")?.as_u64()?;
                (item.as_str(), None)
            }
            Action::Set { item, .. } | Action::Clear { item, .. } => {
                let actual = self.item(value)?;
                (actual == item).then_some((actual, None))?
            }
        };
        Some(Acknowledgement {
            project: self.project.clone(),
            item: item.into(),
            content: content.map(str::to_owned),
        })
    }

    fn item<'a>(&self, value: &'a Value) -> Option<&'a str> {
        let project = value.pointer("/item/project/id")?.as_str()?;
        let item = value.pointer("/item/id")?.as_str()?;
        (project == self.project && node(item)).then_some(item)
    }
}

impl Action {
    fn valid(&self) -> bool {
        match self {
            Self::Add { content } => node(content),
            Self::Remove { item } => node(item),
            Self::Move { item, after } => {
                node(item) && after.as_ref().is_none_or(|id| node(id) && id != item)
            }
            Self::Set { item, field, value } => node(item) && node(field) && value.valid(),
            Self::Clear { item, field } => node(item) && node(field),
        }
    }

    fn request(&self) -> (Mutation, Value) {
        match self {
            Self::Add { content } => (
                Mutation::new(
                    "addProjectV2ItemById",
                    "AddProjectV2ItemByIdInput",
                    "item { id project { id } content { ... on Node { id } } }",
                ),
                json!({"contentId":content}),
            ),
            Self::Remove { item } => (
                Mutation::new(
                    "deleteProjectV2Item",
                    "DeleteProjectV2ItemInput",
                    "deletedItemId",
                ),
                json!({"itemId":item}),
            ),
            Self::Move { item, after } => (
                Mutation::new(
                    "updateProjectV2ItemPosition",
                    "UpdateProjectV2ItemPositionInput",
                    "items { totalCount }",
                ),
                json!({"itemId":item,"afterId":after}),
            ),
            Self::Set { item, field, value } => (
                Mutation::new(
                    "updateProjectV2ItemFieldValue",
                    "UpdateProjectV2ItemFieldValueInput",
                    "item:projectV2Item { id project { id } }",
                ),
                json!({"itemId":item,"fieldId":field,"value":value}),
            ),
            Self::Clear { item, field } => (
                Mutation::new(
                    "clearProjectV2ItemFieldValue",
                    "ClearProjectV2ItemFieldValueInput",
                    "item:projectV2Item { id project { id } }",
                ),
                json!({"itemId":item,"fieldId":field}),
            ),
        }
    }
}

impl Field {
    fn valid(&self) -> bool {
        match self {
            Self::Text(text) => text.len() <= 65_536,
            Self::Number(number) => number.is_finite(),
            Self::Date(value) => date(value),
            Self::Select(id) | Self::Iteration(id) => node(id),
            Self::Multiple(ids) => {
                ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
                    && ids.iter().all(|id| node(id))
            }
        }
    }
}
