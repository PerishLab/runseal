pub mod activity;
pub mod catalogue;
pub mod board {
    pub use super::activity::{Detail, Section, Slice};
    pub use super::definition::{Catalog, Open, Origins};
    pub use super::value::{Account, Subject};
}
mod change;
pub mod columns;
mod definition;
mod schema;
mod structure;
pub mod view;
pub use definition::{Choice, Definition, Iteration, Schedule};
pub use schema::{Alternative, Calendar, Color, Descriptor, Period, Schema, Shape};
pub use structure::{Confirmation, Organization, Overview, Settings, Structure};
mod value;
pub use change::{Acknowledgement, Action, Change, Field};
pub use value::Entry;

use super::{
    Call, Cause, Client, Error, Identity, Page, Receipt, Window, field, graph, node, page, valid,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use std::collections::BTreeSet;

const PROJECTS: &str = "query RunsealProjects($owner:String!,$size:Int!,$after:String) { organization(login:$owner) { id login url kind:__typename list:projectsV2(first:$size,after:$after) { total:totalCount rows:nodes { id kind:__typename number title url public updated:updatedAt owner { kind:__typename ... on Organization { id login } } } page:pageInfo { more:hasNextPage cursor:endCursor } } } }";
const ITEMS: &str = "query RunsealItems($owner:String!,$number:Int!,$size:Int!,$after:String) { organization(login:$owner) { login projectV2(number:$number) { id number list:items(first:$size,after:$after) { total:totalCount rows:nodes { id kind:type archived:isArchived updated:updatedAt project { id } content { kind:__typename ... on Issue { id number title url repository { id name:nameWithOwner } } ... on PullRequest { id number title url repository { id name:nameWithOwner } } ... on DraftIssue { id title } } } page:pageInfo { more:hasNextPage cursor:endCursor } } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub archived: bool,
    pub updated: String,
    pub project: Identity,
    pub content: Option<Content>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Content {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub number: Option<u32>,
    pub url: Option<String>,
    pub repository: Option<Seat>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Seat {
    pub id: String,
    pub name: String,
}

impl Client {
    pub fn items(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<Item>>, Error> {
        let input = selection(owner, number, window)?;
        self.wire
            .send(&self.bearer, Call::Query(graph(ITEMS, input)), |value| {
                let project = connection(&value, owner, number)?;
                let observed: Page<Item> = page::page(project, window)?;
                observed
                    .rows
                    .iter()
                    .all(|item| item.matches(&observed.target))
                    .then_some(observed)
            })
    }
}

impl Item {
    fn matches(&self, scope: &str) -> bool {
        if !node(&self.id) || self.project.id != scope || self.updated.is_empty() {
            return false;
        }
        self.content
            .as_ref()
            .is_none_or(|item| item.matches(&self.kind))
    }
}

impl Content {
    fn matches(&self, kind: &str) -> bool {
        if !node(&self.id) {
            return false;
        }
        match (kind, self.kind.as_str()) {
            ("ISSUE", "Issue") => self.resource("issues"),
            ("PULL_REQUEST", "PullRequest") => self.resource("pull"),
            ("DRAFT_ISSUE", "DraftIssue") => self.number.is_none() && self.repository.is_none(),
            _ => false,
        }
    }

    fn resource(&self, path: &str) -> bool {
        let (Some(number), Some(repository), Some(url)) =
            (self.number, &self.repository, &self.url)
        else {
            return false;
        };
        if number == 0 || !node(&repository.id) || !repository.canonical() {
            return false;
        }
        let expected = format!("https://github.com/{}/{path}/{number}", repository.name);
        url.eq_ignore_ascii_case(&expected)
    }
}

impl Seat {
    fn canonical(&self) -> bool {
        self.name
            .split_once('/')
            .is_some_and(|(owner, name)| valid(owner, false) && valid(name, true))
    }
}

pub(super) fn variables(owner: &str, number: u32) -> Result<Value, Error> {
    if !valid(owner, false) || number == 0 || number > i32::MAX as u32 {
        return Err(Error::local(Cause::Input));
    }
    Ok(json!({"owner":owner, "number":number}))
}

fn selection(owner: &str, number: u32, window: &Window) -> Result<Value, Error> {
    let mut input = variables(owner, number)?;
    input["size"] = json!(window.size);
    input["after"] = json!(window.after);
    Ok(input)
}

fn connection<'a>(value: &'a Value, owner: &str, number: u32) -> Option<&'a Value> {
    let scope = value.pointer("/data/organization")?;
    let login = scope.get("login")?.as_str()?;
    let project = scope.get("projectV2")?;
    let actual = project.get("number")?.as_u64()?;
    (login.eq_ignore_ascii_case(owner) && actual == u64::from(number)).then_some(project)
}

fn mutation(name: &str, kind: &str, output: &str) -> String {
    format!(
        "mutation RunsealProject($input:{kind}!) {{ change:{name}(input:$input) {{ {output} }} }}"
    )
}

fn date(value: &str) -> bool {
    use time::{Date, format_description::well_known::Iso8601};
    value.len() == 10 && Date::parse(value, &Iso8601::DEFAULT).is_ok()
}

pub mod inventory {
    use super::{Organization, Page};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Projects {
        pub owner: Organization,
        pub url: String,
        pub projects: Page<Project>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Project {
        pub id: String,
        pub kind: String,
        pub number: u32,
        pub title: String,
        pub url: String,
        pub public: bool,
        pub updated: String,
        pub owner: Organization,
    }
}

use inventory::{Project, Projects};

impl Client {
    pub fn projects(&self, owner: &str, window: &Window) -> Result<Receipt<Projects>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Query(graph(PROJECTS, variables)),
            |value| inventory(&value, owner, window),
        )
    }
}

fn inventory(value: &Value, requested: &str, window: &Window) -> Option<Projects> {
    let scope = value.pointer("/data/organization")?;
    let owner = Organization::deserialize(scope).ok()?;
    let url: String = field(scope, "/url")?;
    if owner.kind != "Organization"
        || !owner.login.eq_ignore_ascii_case(requested)
        || url != format!("https://github.com/{}", owner.login)
    {
        return None;
    }
    let projects: Page<Project> = page::page(scope, window)?;
    let count = projects.rows.len() as u32;
    match (&window.after, &projects.next) {
        (_, Some(_)) if count >= projects.total => return None,
        (None, None) if count != projects.total => return None,
        _ => {}
    }
    let mut ids = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    projects
        .rows
        .iter()
        .all(|row| row.matches(&owner) && ids.insert(&row.id) && numbers.insert(row.number))
        .then_some(Projects {
            owner,
            url,
            projects,
        })
}

impl Project {
    fn matches(&self, owner: &Organization) -> bool {
        let record = self.kind == "ProjectV2" && node(&self.id) && self.number > 0;
        let organization = self.owner.kind == "Organization"
            && self.owner.id == owner.id
            && self.owner.login.eq_ignore_ascii_case(&owner.login);
        let route = format!(
            "https://github.com/orgs/{}/projects/{}",
            owner.login, self.number
        );
        let metadata = self.url == route && !self.updated.is_empty();
        record && organization && metadata
    }
}
