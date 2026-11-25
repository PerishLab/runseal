use crate::github::{Cause, Client, Error, Number, Window, records::Draft};
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
    #[command(about = "Create an Issue with a named type using repository permissions")]
    File {
        owner: String,
        name: String,
        #[arg(long)]
        title: String,
        #[arg(long = "body-file")]
        body: PathBuf,
        #[arg(long)]
        kind: String,
    },
    #[command(about = "Read an exact Issue with its native author and labels")]
    Show(super::delivery::Spot),
    #[command(about = "Read the exact Issue and one page of each related connection")]
    Snapshot(Snapshot),
    #[command(about = "Read an exact Issue comment with its author type and update identity")]
    Document {
        #[command(flatten)]
        target: super::delivery::Spot,
        id: u64,
    },
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
    #[command(about = "Search GitHub Issues with bounded pagination")]
    Search {
        query: String,
        #[arg(long, default_value_t = 30)]
        size: u8,
        #[arg(long)]
        after: Option<String>,
    },
    #[command(about = "Classify repository numbers as open or closed Issues or absent")]
    Classify {
        owner: String,
        name: String,
        #[arg(required = true)]
        numbers: Vec<u32>,
    },
    #[command(about = "Read one page of REST Issue field values for an exact Issue")]
    Attached {
        #[command(flatten)]
        target: super::delivery::Spot,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 100)]
        size: u8,
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
            Action::File {
                owner,
                name,
                title,
                body,
                kind,
            } => {
                let draft = Draft::new(&title, &super::read(body)?, &kind)?;
                encode(client.file(&owner, &name, &draft)?)
            }
            Action::Show(target) => encode(client.ticket(&target.target()?)?),
            Action::Snapshot(input) => input.execute(client),
            Action::Document { target, id } => encode(client.document(&target.target()?, id)?),
            Action::Comments { target, page, size } => {
                encode(client.discussion(&target.target()?, Number::new(page, size)?)?)
            }
            Action::Entry { target, id } => encode(client.entry(&target.target()?, id)?),
            Action::Complete { node, body } => encode(client.complete(&node, &super::read(body)?)?),
            Action::Search { query, size, after } => {
                let window = Window::new(size, after.as_deref())?;
                encode(client.search(&query, &window)?)
            }
            Action::Classify {
                owner,
                name,
                numbers,
            } => encode(client.classify(&owner, &name, &numbers)?),
            Action::Attached { target, page, size } => {
                encode(client.attached(&target.target()?, Number::new(page, size)?)?)
            }
        }
    }
}

fn encode(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::local(Cause::Shape))
}

#[derive(Args)]
struct Snapshot {
    #[command(flatten)]
    target: super::delivery::Spot,
    #[arg(long, default_value_t = 30)]
    size: u8,
    #[command(flatten)]
    cursors: crate::github::snapshot::Cursors,
}

impl Snapshot {
    fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.snapshot(&self.target.target()?, self.size, &self.cursors)?)
    }
}
