use super::{Call, Cause, Client, Error, Receipt, columns::Locator, graph, mutation, node};
use crate::github::{Identity, Outcome, field};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Table,
    Board,
    Roadmap,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub name: String,
    pub layout: Layout,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "visible_fields", skip_serializing_if = "Option::is_none")]
    pub visible: Option<Vec<u64>>,
    #[serde(rename = "group_by", skip_serializing_if = "Option::is_none")]
    pub group: Option<Vec<u64>>,
    #[serde(rename = "vertical_group_by", skip_serializing_if = "Option::is_none")]
    pub vertical: Option<Vec<u64>>,
    #[serde(rename = "sort_by", skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<(u64, Direction)>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Created {
    pub id: u64,
    pub number: u32,
    #[serde(rename = "node_id")]
    pub node: String,
    pub name: String,
    pub layout: Layout,
    #[serde(rename = "project_url")]
    pub project: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Change {
    Update {
        view: String,
        name: Option<String>,
        layout: Option<Layout>,
        filter: Option<String>,
        visible: Option<Vec<String>>,
    },
    Remove {
        view: String,
    },
}

impl Create {
    pub fn submit(&self, client: &Client, location: &Locator) -> Result<Receipt<Created>, Error> {
        if !self.valid() {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("{}/views", location.route());
        let mut input = json!(self);
        if self.layout == Layout::Roadmap {
            input
                .as_object_mut()
                .expect("view fields")
                .remove("visible_fields");
        }
        let mut receipt = client.wire.inspect(
            &client.bearer,
            Call::Recent(&path, Some(input)),
            |value, response| {
                if response.status != 201 {
                    return None;
                }
                let found: Created = field(&value, "/value")?;
                found.matches(self, location).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Change {
    pub fn submit(&self, client: &Client) -> Result<Receipt<Identity>, Error> {
        let (query, input) = self.request().ok_or_else(|| Error::local(Cause::Input))?;
        let mut receipt = client.wire.send(
            &client.bearer,
            Call::Mutation(graph(&query, json!({"input":input}))),
            |value| {
                let found: Identity = field(&value, "/data/change/projectV2View")?;
                (self.id() == found.id).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Create {
    fn valid(&self) -> bool {
        if self.name.trim().is_empty()
            || !columns(&self.visible, false)
            || !columns(&self.group, true)
        {
            return false;
        }
        if !columns(&self.vertical, true)
            || self
                .sort
                .as_ref()
                .is_some_and(|sort| sort.iter().any(|value| value.0 == 0))
        {
            return false;
        }
        if self.layout == Layout::Roadmap
            && self
                .visible
                .as_ref()
                .is_some_and(|fields| !fields.is_empty())
        {
            return false;
        }
        self.layout == Layout::Board || self.vertical.as_ref().is_none_or(Vec::is_empty)
    }
}

impl Created {
    fn matches(&self, create: &Create, location: &Locator) -> bool {
        if self.id == 0 || self.number == 0 || !node(&self.node) {
            return false;
        }
        self.name == create.name
            && self.layout == create.layout
            && location.address(&self.project, "")
    }
}

impl Change {
    fn id(&self) -> &str {
        match self {
            Self::Update { view, .. } | Self::Remove { view } => view,
        }
    }

    fn request(&self) -> Option<(String, Value)> {
        if !node(self.id()) {
            return None;
        }
        let output = "projectV2View { id }";
        let mut input = json!({"viewId":self.id()});
        let Self::Update {
            name,
            layout,
            filter,
            visible,
            ..
        } = self
        else {
            return Some((
                mutation("deleteProjectV2View", "DeleteProjectV2ViewInput", output),
                input,
            ));
        };
        if let Some(name) = name {
            if name.trim().is_empty() {
                return None;
            }
            input["name"] = json!(name);
        }
        if let Some(layout) = layout {
            input["layout"] = json!(layout.native());
        }
        if let Some(filter) = filter {
            input["filter"] = json!(filter);
        }
        if let Some(visible) = visible {
            if !visible.iter().all(|id| node(id)) {
                return None;
            }
            input["configuration"] = json!({"visibleFieldIds":visible});
        }
        if input.as_object()?.len() == 1 {
            return None;
        }
        Some((
            mutation("updateProjectV2View", "UpdateProjectV2ViewInput", output),
            input,
        ))
    }
}

impl Layout {
    fn native(self) -> &'static str {
        match self {
            Self::Table => "TABLE_LAYOUT",
            Self::Board => "BOARD_LAYOUT",
            Self::Roadmap => "ROADMAP_LAYOUT",
        }
    }
}

fn columns(values: &Option<Vec<u64>>, single: bool) -> bool {
    values
        .as_ref()
        .is_none_or(|values| (!single || values.len() <= 1) && !values.contains(&0))
}
