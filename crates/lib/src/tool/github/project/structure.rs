use super::{
    Call, Cause, Client, Error, Page, Receipt, Seat, Window, connection, graph, mutation, node,
    page, selection, valid, variables,
};
use crate::github::{Outcome, field};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const METADATA: &str = "id number title url updated:updatedAt closed public writable:viewerCanUpdate description:shortDescription readme owner { kind:__typename ... on Organization { id login } }";
const REPOSITORIES: &str = "query RunsealRepositories($owner:String!,$number:Int!,$size:Int!,$after:String) { organization(login:$owner) { login projectV2(number:$number) { id number list:repositories(first:$size,after:$after) { total:totalCount rows:nodes { id name:nameWithOwner } page:pageInfo { more:hasNextPage cursor:endCursor } } } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Overview {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub url: String,
    pub updated: String,
    pub closed: bool,
    pub public: bool,
    pub writable: bool,
    pub description: Option<String>,
    pub readme: Option<String>,
    pub owner: Organization,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Organization {
    pub id: String,
    pub login: String,
    pub kind: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "shortDescription", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Structure {
    Create {
        owner: String,
        title: String,
    },
    Configure {
        project: String,
        settings: Settings,
    },
    Relate {
        project: String,
        repository: String,
        present: bool,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Confirmation {
    Project {
        project: Box<Overview>,
    },
    Repository {
        project: String,
        repository: String,
        present: bool,
    },
}

impl Client {
    pub fn overview(&self, owner: &str, number: u32) -> Result<Receipt<Overview>, Error> {
        let input = variables(owner, number)?;
        let query = format!(
            "query RunsealOverview($owner:String!,$number:Int!) {{ organization(login:$owner) {{ login projectV2(number:$number) {{ {METADATA} }} }} }}"
        );
        self.wire
            .send(&self.bearer, Call::Query(graph(&query, input)), |value| {
                let scope = connection(&value, owner, number)?;
                let found = Overview::deserialize(scope).ok()?;
                let verified = found.valid() && found.owner.login.eq_ignore_ascii_case(owner);
                verified.then_some(found)
            })
    }

    pub fn repositories(
        &self,
        owner: &str,
        number: u32,
        window: &Window,
    ) -> Result<Receipt<Page<Seat>>, Error> {
        let input = selection(owner, number, window)?;
        self.wire.send(
            &self.bearer,
            Call::Query(graph(REPOSITORIES, input)),
            |value| {
                let scope = connection(&value, owner, number)?;
                let found: Page<Seat> = page::page(scope, window)?;
                found
                    .rows
                    .iter()
                    .all(|repo| node(&repo.id) && repo.canonical())
                    .then_some(found)
            },
        )
    }

    pub fn structure(&self, command: &Structure) -> Result<Receipt<Confirmation>, Error> {
        let (query, input) = command
            .request()
            .ok_or_else(|| Error::local(Cause::Input))?;
        let call = Call::Mutation(graph(&query, json!({"input":input})));
        let mut receipt = self
            .wire
            .send(&self.bearer, call, |value| command.confirm(&value))?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Structure {
    fn request(&self) -> Option<(String, Value)> {
        let metadata = format!("project:projectV2 {{ {METADATA} }}");
        let (name, kind, input) = match self {
            Self::Create { owner, title } if node(owner) && named(title) => (
                "createProjectV2",
                "CreateProjectV2Input",
                json!({"ownerId": owner, "title": title}),
            ),
            Self::Configure { project, settings } if node(project) => (
                "updateProjectV2",
                "UpdateProjectV2Input",
                settings.input(project)?,
            ),
            Self::Relate {
                project,
                repository,
                present,
            } if node(project) && node(repository) => {
                let (name, kind) = if *present {
                    (
                        "linkProjectV2ToRepository",
                        "LinkProjectV2ToRepositoryInput",
                    )
                } else {
                    (
                        "unlinkProjectV2FromRepository",
                        "UnlinkProjectV2FromRepositoryInput",
                    )
                };
                let output = "repository { id }";
                return Some((
                    mutation(name, kind, output),
                    json!({"projectId":project,"repositoryId":repository}),
                ));
            }
            _ => return None,
        };
        Some((mutation(name, kind, &metadata), input))
    }

    fn confirm(&self, value: &Value) -> Option<Confirmation> {
        if let Self::Relate {
            project,
            repository,
            present,
        } = self
        {
            let actual = value.pointer("/data/change/repository/id")?.as_str()?;
            return (actual == repository).then(|| Confirmation::Repository {
                project: project.clone(),
                repository: repository.clone(),
                present: *present,
            });
        }
        let found: Overview = field(value, "/data/change/project")?;
        let matches = match self {
            Self::Create { owner, title } => found.owner.id == *owner && found.title == *title,
            Self::Configure { project, settings } => {
                found.id == *project && settings.matches(&found)
            }
            Self::Relate { .. } => false,
        };
        (matches && found.valid()).then(|| Confirmation::Project {
            project: Box::new(found),
        })
    }
}

impl Overview {
    fn valid(&self) -> bool {
        if !node(&self.id) || self.number == 0 || self.updated.is_empty() {
            return false;
        }
        let owner = &self.owner;
        if owner.kind != "Organization" || !node(&owner.id) || !valid(&owner.login, false) {
            return false;
        }
        self.url.eq_ignore_ascii_case(&format!(
            "https://github.com/orgs/{}/projects/{}",
            owner.login, self.number
        ))
    }
}

impl Settings {
    fn input(&self, project: &str) -> Option<Value> {
        if self.title.as_deref().is_some_and(|title| !named(title)) {
            return None;
        }
        let mut input = serde_json::to_value(self).ok()?;
        if input.as_object()?.is_empty() {
            return None;
        }
        input["projectId"] = json!(project);
        Some(input)
    }

    fn matches(&self, project: &Overview) -> bool {
        let matches = [
            self.title
                .as_ref()
                .is_none_or(|title| title == &project.title),
            text(&self.description, &project.description),
            text(&self.readme, &project.readme),
            self.public.is_none_or(|public| public == project.public),
            self.closed.is_none_or(|closed| closed == project.closed),
        ];
        matches.into_iter().all(|matched| matched)
    }
}

fn text(expected: &Option<String>, actual: &Option<String>) -> bool {
    expected
        .as_deref()
        .is_none_or(|text| text == actual.as_deref().unwrap_or_default())
}

fn named(title: &str) -> bool {
    !title.trim().is_empty()
}
