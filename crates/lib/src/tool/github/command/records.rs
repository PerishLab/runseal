use crate::github::{Cause, Client, Error, Number};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Args)]
pub(super) struct Issues {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    #[command(about = "Read the authenticated GitHub actor")]
    Viewer,
    #[command(about = "Read one bounded page of open repository Issues")]
    Open {
        owner: String,
        name: String,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 30)]
        size: u8,
    },
    #[command(about = "Read an exact Issue with its native author and labels")]
    Show(super::delivery::Spot),
    #[command(about = "Read one page of Issue comments with their native authors")]
    Comments {
        #[command(flatten)]
        target: super::delivery::Spot,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 30)]
        size: u8,
    },
    #[command(about = "Read a comment bound to its exact Issue and numeric ID")]
    Entry {
        #[command(flatten)]
        target: super::delivery::Spot,
        id: u64,
    },
    #[command(about = "Set the body and close an exact Issue as completed in one update")]
    Complete {
        node: String,
        #[arg(long = "body-file")]
        body: PathBuf,
    },
}

impl Issues {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        match self.action {
            Action::Viewer => encode(client.viewer()?),
            Action::Open {
                owner,
                name,
                page,
                size,
            } => encode(client.issues(&owner, &name, page, size)?),
            Action::Show(target) => encode(client.ticket(&target.target()?)?),
            Action::Comments { target, page, size } => {
                encode(client.discussion(&target.target()?, Number::new(page, size)?)?)
            }
            Action::Entry { target, id } => encode(client.entry(&target.target()?, id)?),
            Action::Complete { node, body } => encode(client.complete(&node, &super::read(body)?)?),
        }
    }
}

fn encode(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::local(Cause::Shape))
}
