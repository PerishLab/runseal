use super::{Spot, read};
use crate::github::{Cause, Client, Compare, Edit, Error, Proposal, Search, State};
use crate::github::{Number, Window, lifecycle};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Args)]
struct Branches {
    owner: String,
    name: String,
    #[arg(long)]
    head: String,
    #[arg(long)]
    base: String,
}

impl Branches {
    fn compare(&self) -> Result<Compare, Error> {
        Compare::new(&self.head, &self.base)
    }
}

#[derive(Args)]
pub(super) struct Selection {
    #[command(flatten)]
    repo: Branches,
    #[arg(long, value_enum, default_value = "open")]
    state: State,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
}

impl Selection {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let search = Search::new(self.repo.compare()?, self.state, self.page, self.size)?;
        let response = client.pulls(&self.repo.owner, &self.repo.name, &search)?;
        serde_json::to_value(response).map_err(|_| Error::local(Cause::Shape))
    }
}

#[derive(Args)]
pub(super) struct Submission {
    #[command(flatten)]
    repo: Branches,
    #[arg(long)]
    title: String,
    #[arg(long = "body-file")]
    body: Option<PathBuf>,
}

impl Submission {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let body = self.body.map(read).transpose()?;
        let proposal = Proposal::new(self.repo.compare()?, &self.title, body.as_deref())?;
        let response = client.propose(&self.repo.owner, &self.repo.name, &proposal)?;
        serde_json::to_value(response).map_err(|_| Error::local(Cause::Shape))
    }
}

#[derive(Args)]
pub(super) struct Revision {
    #[command(flatten)]
    place: Spot,
    #[arg(long)]
    title: String,
    #[arg(long = "body-file")]
    body: PathBuf,
}

impl Revision {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let edit = Edit::new(&self.title, &read(self.body)?)?;
        let response = client.edit(&self.place.target()?, &edit)?;
        serde_json::to_value(response).map_err(|_| Error::local(Cause::Shape))
    }
}

#[derive(Args)]
pub(super) struct Lifecycle {
    #[command(subcommand)]
    deed: Deed,
}

#[derive(Subcommand)]
enum Deed {
    #[command(about = "Read merge readiness with the exact pull head SHA")]
    Readiness(Spot),
    #[command(about = "Read a page of checks and status contexts for an exact pull head")]
    Rollup {
        #[command(flatten)]
        place: Spot,
        #[arg(long)]
        head: String,
        #[arg(long, default_value_t = 30)]
        size: u8,
        #[arg(long)]
        after: Option<String>,
    },
    #[command(about = "Replace only the body of an exact pull")]
    Body {
        #[command(flatten)]
        place: Spot,
        #[arg(long = "body-file")]
        body: PathBuf,
    },
    #[command(about = "Close an exact pull without merging it")]
    Close(Spot),
    #[command(about = "List one page for an exact head, optionally scoped to a base")]
    List {
        owner: String,
        name: String,
        #[arg(long)]
        head: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long, value_enum, default_value = "all")]
        state: State,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 30)]
        size: u8,
    },
}

impl Lifecycle {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        match self.deed {
            Deed::Readiness(place) => encode(client.readiness(&place.target()?)?),
            Deed::Rollup {
                place,
                head,
                size,
                after,
            } => {
                let window = Window::new(size, after.as_deref())?;
                encode(client.rollup(&place.target()?, &head, &window)?)
            }
            Deed::Body { place, body } => {
                let update = lifecycle::Update::Body(read(body)?);
                encode(client.revise(&place.target()?, &update)?)
            }
            Deed::Close(place) => {
                encode(client.revise(&place.target()?, &lifecycle::Update::Close)?)
            }
            Deed::List {
                owner,
                name,
                head,
                base,
                state,
                page,
                size,
            } => {
                let selection = lifecycle::Selection::new(
                    &head,
                    base.as_deref(),
                    state,
                    Number::new(page, size)?,
                )?;
                encode(client.listing(&owner, &name, &selection)?)
            }
        }
    }
}

fn encode(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::local(Cause::Shape))
}
