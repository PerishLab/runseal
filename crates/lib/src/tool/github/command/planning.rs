use super::delivery::{decode, encode, read};
use crate::github::{Client, Error, Target, Window, columns::Locator, view};
use clap::{Args, Subcommand};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Args)]
pub(super) struct Definitions {
    owner: String,
    #[command(flatten)]
    page: Pagination,
}

#[derive(Args)]
pub(super) struct Observations {
    owner: String,
    name: String,
    number: u32,
    #[command(flatten)]
    page: Pagination,
}

#[derive(Args)]
pub(super) struct Phase {
    #[arg(long = "phase-file")]
    phase: PathBuf,
}

#[derive(Args)]
struct Pagination {
    #[arg(long, default_value_t = 20)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Pagination {
    fn window(&self) -> Result<Window, Error> {
        Window::new(self.size, self.after.as_deref())
    }
}

impl Definitions {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.definitions(&self.owner, &self.page.window()?)?)
    }
}

impl Observations {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let target = Target::new(&self.owner, &self.name, self.number)?;
        encode(client.observations(&target, &self.page.window()?)?)
    }
}

impl Phase {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.select(&decode(self.phase)?)?)
    }
}

#[derive(Args)]
pub(super) struct Project {
    #[command(subcommand)]
    action: Action,
}

#[derive(Args)]
struct Scope {
    owner: String,
    number: u32,
}

#[derive(Args)]
struct Located {
    #[command(flatten)]
    project: Scope,
    #[arg(long = "organization-id")]
    organization: u64,
}

impl Located {
    fn locator(&self) -> Result<Locator, Error> {
        Locator::new(&self.project.owner, self.project.number, self.organization)
    }
}

#[derive(Subcommand)]
enum Action {
    Views {
        #[command(flatten)]
        scope: Scope,
        #[command(flatten)]
        page: Pagination,
    },
    Workflows {
        #[command(flatten)]
        scope: Scope,
        #[command(flatten)]
        page: Pagination,
    },
    Updates {
        #[command(flatten)]
        scope: Scope,
        #[command(flatten)]
        page: Pagination,
    },
    Columns {
        #[command(flatten)]
        scope: Located,
        #[command(flatten)]
        page: Pagination,
    },
    #[command(name = "create-view")]
    Create {
        #[command(flatten)]
        scope: Located,
        #[arg(long = "view-file")]
        view: PathBuf,
    },
    #[command(name = "change-view")]
    Change {
        #[arg(long = "view-file")]
        view: PathBuf,
    },
    #[command(about = "Remove one exact Project workflow")]
    Retire { project: String, workflow: String },
    #[command(about = "Post one Project status update")]
    Record {
        project: String,
        #[arg(long = "body-file")]
        body: PathBuf,
    },
}

impl Project {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        match self.action {
            Action::Views { scope, page } => {
                encode(client.views(&scope.owner, scope.number, &page.window()?)?)
            }
            Action::Workflows { scope, page } => {
                encode(client.workflows(&scope.owner, scope.number, &page.window()?)?)
            }
            Action::Updates { scope, page } => {
                encode(client.updates(&scope.owner, scope.number, &page.window()?)?)
            }
            Action::Columns { scope, page } => {
                encode(client.columns(&scope.locator()?, &page.window()?)?)
            }
            Action::Create { scope, view } => {
                encode(decode::<view::Create>(view)?.submit(client, &scope.locator()?)?)
            }
            Action::Change { view } => encode(decode::<view::Change>(view)?.submit(client)?),
            Action::Retire { project, workflow } => encode(client.retire(&project, &workflow)?),
            Action::Record { project, body } => encode(client.record(&project, &read(body)?)?),
        }
    }
}
