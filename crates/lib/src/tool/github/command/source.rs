use crate::github::{Cause, Client, Error};
use clap::Args;
use serde::Serialize;
use serde_json::Value;

#[derive(Args)]
pub(super) struct Tags {
    owner: String,
    name: String,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
}

impl Tags {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.tags(&self.owner, &self.name, self.page, self.size)?)
    }
}

#[derive(Args)]
pub(super) struct Pointer {
    owner: String,
    name: String,
    #[arg(value_enum)]
    kind: Namespace,
    reference: String,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Namespace {
    Branch,
    Tag,
}

impl Pointer {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        use crate::github::Reference;
        let selection = match self.kind {
            Namespace::Branch => Reference::Branch(self.reference),
            Namespace::Tag => Reference::Tag(self.reference),
        };
        encode(client.reference(&self.owner, &self.name, &selection)?)
    }
}

#[derive(Args)]
pub(super) struct Annotation {
    owner: String,
    name: String,
    id: String,
}

impl Annotation {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.annotation(&self.owner, &self.name, &self.id)?)
    }
}

#[derive(Args)]
pub(super) struct Blob {
    #[command(flatten)]
    object: Selection,
}

#[derive(Args)]
pub(super) struct Tree {
    #[command(flatten)]
    object: Selection,
    #[arg(long)]
    recursive: bool,
}

#[derive(Args)]
struct Selection {
    owner: String,
    name: String,
    id: String,
}

impl Tree {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let target = self.object;
        encode(client.tree(&target.owner, &target.name, &target.id, self.recursive)?)
    }
}

impl Blob {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let target = self.object;
        encode(client.blob(&target.owner, &target.name, &target.id)?)
    }
}

fn encode(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::local(Cause::Shape))
}

#[derive(Args)]
pub(super) struct Branch {
    owner: String,
    name: String,
    branch: String,
}

impl Branch {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.branch(&self.owner, &self.name, &self.branch)?)
    }
}

#[derive(Args)]
pub(super) struct Permission {
    owner: String,
    name: String,
    login: String,
}

impl Permission {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.permission(&self.owner, &self.name, &self.login)?)
    }
}

#[derive(Args)]
pub(super) struct Contents {
    owner: String,
    name: String,
    commit: String,
    path: String,
}

impl Contents {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.contents(&self.owner, &self.name, &self.commit, &self.path)?)
    }
}

#[derive(Args)]
pub(super) struct Matching {
    owner: String,
    name: String,
    prefix: String,
}

impl Matching {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.matching(&self.owner, &self.name, &self.prefix)?)
    }
}

#[derive(Args)]
pub(super) struct Releases {
    #[command(subcommand)]
    action: Publication,
}

#[derive(clap::Subcommand)]
enum Publication {
    Tag {
        owner: String,
        name: String,
        tag: String,
    },
    Asset {
        owner: String,
        name: String,
        id: u64,
    },
    #[command(about = "Download a release asset with expected byte length and SHA-256")]
    Download {
        owner: String,
        name: String,
        id: u64,
        #[arg(long)]
        size: u64,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        output: std::path::PathBuf,
    },
}

impl Releases {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        match self.action {
            Publication::Tag { owner, name, tag } => encode(client.release(&owner, &name, &tag)?),
            Publication::Asset { owner, name, id } => encode(client.asset(&owner, &name, id)?),
            Publication::Download {
                owner,
                name,
                id,
                size,
                digest,
                output,
            } => {
                let download = crate::github::Download::new(id, size, &digest)?;
                encode(client.fetch(&owner, &name, &download, &output)?)
            }
        }
    }
}

#[derive(Args)]
pub(super) struct Repository {
    owner: String,
    name: String,
}

impl Repository {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.repository(&self.owner, &self.name)?)
    }
}

#[derive(Args)]
pub(super) struct Project {
    owner: String,
    number: u32,
}

impl Project {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.project(&self.owner, self.number)?)
    }
}
