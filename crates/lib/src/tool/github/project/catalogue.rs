use super::{Call, Client, Error, Page, Receipt, Window, connection, graph, node, page, selection};
use crate::github::Identity;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const REFERENCE: &str =
    "kind:__typename ... on ProjectV2FieldCommon { id name type:dataType issue:isIssueField }";
const FRAME: &str = "page:pageInfo { more:hasNextPage cursor:endCursor }";

#[derive(Debug, Deserialize, Serialize)]
pub struct View {
    pub id: String,
    pub number: u32,
    pub name: String,
    pub layout: String,
    pub filter: Option<String>,
    pub updated: String,
    pub project: Identity,
    pub configuration: Option<Configuration>,
    pub group: Segment<Reference>,
    pub vertical: Segment<Reference>,
    pub sort: Segment<Sort>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub visible: Segment<Reference>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Segment<T> {
    pub total: Option<u32>,
    pub rows: Vec<T>,
    pub page: Frame,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Frame {
    pub more: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reference {
    pub kind: String,
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub datatype: Option<String>,
    pub issue: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sort {
    pub direction: String,
    pub field: Reference,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Workflow {
    pub id: String,
    pub number: u32,
    pub name: String,
    pub enabled: bool,
    pub updated: String,
    pub project: Identity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Update {
    pub id: String,
    pub body: Option<String>,
    pub project: Identity,
    pub created: String,
    pub updated: String,
    pub start: Option<String>,
    pub target: Option<String>,
    pub status: Option<String>,
}

pub(super) const UPDATE: &str = "id body created:createdAt updated:updatedAt start:startDate target:targetDate status project { id }";

struct Listing<'a> {
    owner: &'a str,
    number: u32,
    connection: &'a str,
    fields: &'a str,
}

impl Client {
    pub fn views(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<View>>, Error> {
        let fields = format!("id number name layout filter updated:updatedAt project {{ id }}
            configuration {{ visible:visibleFields(first:100) {{ rows:nodes {{ {REFERENCE} }} {FRAME} }} }}
            group:groupByFields(first:100) {{ total:totalCount rows:nodes {{ {REFERENCE} }} {FRAME} }}
            vertical:verticalGroupByFields(first:100) {{ total:totalCount rows:nodes {{ {REFERENCE} }} {FRAME} }}
            sort:sortByFields(first:100) {{ total:totalCount rows:nodes {{ direction field {{ {REFERENCE} }} }} {FRAME} }}");
        self.catalogue(
            Listing {
                owner,
                number,
                connection: "views",
                fields: &fields,
            },
            window,
            View::valid,
        )
    }

    pub fn workflows(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<Workflow>>, Error> {
        let fields = "id number name enabled updated:updatedAt project { id }";
        self.catalogue(
            Listing {
                owner,
                number,
                connection: "workflows",
                fields,
            },
            window,
            Workflow::valid,
        )
    }

    pub fn updates(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<Update>>, Error> {
        self.catalogue(
            Listing {
                owner,
                number,
                connection: "statusUpdates",
                fields: UPDATE,
            },
            window,
            Update::valid,
        )
    }

    fn catalogue<T: DeserializeOwned>(
        &self,
        query: Listing<'_>,
        window: &Window,
        qualify: impl Fn(&T, &str) -> bool,
    ) -> Result<Receipt<Page<T>>, Error> {
        let input = selection(query.owner, query.number, window)?;
        let statement = format!(
            "query RunsealCatalogue($owner:String!,$number:Int!,$size:Int!,$after:String) {{ organization(login:$owner) {{ login projectV2(number:$number) {{ id number list:{}(first:$size,after:$after) {{ total:totalCount rows:nodes {{ {} }} {FRAME} }} }} }} }}",
            query.connection, query.fields
        );
        self.wire.send(
            &self.bearer,
            Call::Query(graph(&statement, input)),
            |value| {
                let scope = connection(&value, query.owner, query.number)?;
                let found: Page<T> = page::page(scope, window)?;
                found
                    .rows
                    .iter()
                    .all(|row| qualify(row, &found.target))
                    .then_some(found)
            },
        )
    }
}

impl View {
    fn valid(&self, project: &str) -> bool {
        if !node(&self.id) || self.number == 0 || self.project.id != project {
            return false;
        }
        let groups = [&self.group, &self.vertical];
        if !groups
            .into_iter()
            .all(|group| group.valid(Reference::valid))
        {
            return false;
        }
        self.sort.valid(|entry| entry.field.valid())
            && self
                .configuration
                .as_ref()
                .is_none_or(|config| config.visible.valid(Reference::valid))
    }
}

impl Workflow {
    fn valid(&self, project: &str) -> bool {
        node(&self.id) && self.number != 0 && self.project.id == project
    }
}

impl Update {
    pub(super) fn valid(&self, project: &str) -> bool {
        if !node(&self.id) || self.project.id != project {
            return false;
        }
        !self.created.is_empty() && !self.updated.is_empty()
    }
}

impl<T> Segment<T> {
    fn valid(&self, record: impl Fn(&T) -> bool) -> bool {
        if self.rows.len() > 100
            || self
                .total
                .is_some_and(|total| (total as usize) < self.rows.len())
        {
            return false;
        }
        if self.page.more && self.page.cursor.as_deref().is_none_or(str::is_empty) {
            return false;
        }
        self.rows.iter().all(record)
    }
}

impl Reference {
    fn valid(&self) -> bool {
        !self.kind.is_empty() && self.id.as_deref().is_none_or(node)
    }
}
