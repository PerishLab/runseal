use super::{Spot, read};
use crate::github::{Cause, Client, Compare, Edit, Error, Proposal, Search, State};
use clap::Args;
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
