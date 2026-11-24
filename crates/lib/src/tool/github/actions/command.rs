use crate::github::command::read;
use crate::github::{Cause, Client, Dispatch, Error, Listing};
use clap::Args;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Args)]
pub(crate) struct Request {
    owner: String,
    name: String,
    workflow: String,
    reference: String,
    #[arg(
        long,
        help = "Read a bounded JSON object of string inputs from this file"
    )]
    inputs: Option<PathBuf>,
}

impl Request {
    pub(crate) fn execute(self, client: &Client) -> Result<Value, Error> {
        let mut inputs = BTreeMap::new();
        if let Some(path) = self.inputs {
            inputs = serde_json::from_str(&read(path)?).map_err(|_| Error::local(Cause::Input))?;
        }
        let request = Dispatch::new(&self.workflow, &self.reference, inputs)?;
        let receipt = client.dispatch(&self.owner, &self.name, &request)?;
        serde_json::to_value(receipt).map_err(|_| Error::local(Cause::Shape))
    }
}

#[derive(Args)]
pub(crate) struct Catalog {
    owner: String,
    name: String,
    id: u64,
    #[arg(long)]
    attempt: u32,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
}

impl Catalog {
    pub(crate) fn execute(self, client: &Client) -> Result<Value, Error> {
        let query = Listing::new(self.id, self.attempt, self.page, self.size)?;
        let receipt = client.jobs(&self.owner, &self.name, query)?;
        serde_json::to_value(receipt).map_err(|_| Error::local(Cause::Shape))
    }
}
