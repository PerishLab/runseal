use crate::github::{
    Cause, Client, Draft, Error, Filter, Inventory, Signal, Target, Verdict, Window,
};
use clap::Args;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Args)]
pub(super) struct Creation {
    repository: String,
    #[arg(long)]
    title: String,
    #[arg(long = "body-file")]
    body: Option<PathBuf>,
    #[arg(long)]
    kind: Option<String>,
}

impl Creation {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let body = self.body.map(read).transpose()?.unwrap_or_default();
        let draft = Draft::new(&self.repository, &self.title, &body, self.kind.as_deref())?;
        encode(client.create(&draft)?)
    }
}

#[derive(Args)]
pub(super) struct Statuses {
    owner: String,
    name: String,
    head: String,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
}

impl Statuses {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let filter = Filter::new(&self.head, self.page, self.size)?;
        encode(client.statuses(&self.owner, &self.name, &filter)?)
    }
}

#[derive(Args)]
pub(super) struct Mark {
    owner: String,
    name: String,
    head: String,
    #[arg(long, value_enum)]
    state: Verdict,
    #[arg(long)]
    context: String,
    #[arg(long)]
    description: Option<String>,
    #[arg(long, value_name = "URL")]
    target: Option<String>,
}

impl Mark {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let signal = Signal::new(
            self.state,
            &self.context,
            self.description.as_deref(),
            self.target.as_deref(),
        )?;
        encode(client.mark(&self.owner, &self.name, &self.head, &signal)?)
    }
}

#[derive(Args)]
pub(super) struct Artifacts {
    owner: String,
    name: String,
    #[arg(long)]
    run: Option<u64>,
    #[arg(long = "name", value_name = "ARTIFACT")]
    label: Option<String>,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
}

impl Artifacts {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let query = Inventory::new(self.run, self.page, self.size)?;
        let query = match self.label {
            Some(label) => query.named(&label)?,
            None => query,
        };
        encode(client.artifacts(&self.owner, &self.name, query)?)
    }
}

pub(super) fn encode(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::local(Cause::Shape))
}

#[derive(Args)]
pub(super) struct Download {
    owner: String,
    name: String,
    id: u64,
    #[arg(long)]
    size: u64,
    #[arg(long)]
    digest: String,
    #[arg(long)]
    output: PathBuf,
}

impl Download {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let query = crate::github::Download::new(self.id, self.size, &self.digest)?;
        encode(client.download(&self.owner, &self.name, &query, &self.output)?)
    }
}

#[derive(Args)]
pub(super) struct Commit {
    owner: String,
    name: String,
    head: String,
}

impl Commit {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.commit(&self.owner, &self.name, &self.head)?)
    }
}

pub(in crate::tool::github) fn read(file: PathBuf) -> Result<String, Error> {
    let bad = |_| Error::local(Cause::Input);
    let mut body = String::new();
    File::open(file)
        .map_err(bad)?
        .take(65_537)
        .read_to_string(&mut body)
        .map_err(bad)?;
    Ok(body)
}

#[derive(Args)]
pub(super) struct Organization {
    owner: String,
    #[arg(long, default_value_t = 30)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Organization {
    pub(super) fn projects(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.projects(&self.owner, &window)?)
    }

    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.kinds(&self.owner, &window)?)
    }
}

#[derive(Args)]
pub(super) struct Items {
    owner: String,
    project: u32,
    #[arg(long, default_value_t = 10)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Items {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.items(&self.owner, self.project, &window)?)
    }

    pub(super) fn fields(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.fields(&self.owner, self.project, &window)?)
    }

    pub(super) fn repositories(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.repositories(&self.owner, self.project, &window)?)
    }
}

#[derive(Args)]
pub(super) struct Change {
    #[arg(long = "change-file", value_name = "FILE")]
    change: PathBuf,
}

impl Change {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.change(&decode(self.change)?)?)
    }
}

#[derive(Args)]
pub(super) struct Values {
    project: String,
    item: String,
    #[arg(long, default_value_t = 20)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Values {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        let window = Window::new(self.size, self.after.as_deref())?;
        encode(client.values(&self.project, &self.item, &window)?)
    }
}

#[derive(Args)]
pub(super) struct Spot {
    owner: String,
    name: String,
    number: u32,
}

#[derive(Args)]
pub(super) struct Span {
    #[command(flatten)]
    place: Spot,
    #[arg(long, default_value_t = 10)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Spot {
    pub(super) fn target(&self) -> Result<Target, Error> {
        Target::new(&self.owner, &self.name, self.number)
    }
}

impl Span {
    pub(super) fn parts(&self) -> Result<(Target, Window), Error> {
        Ok((
            self.place.target()?,
            Window::new(self.size, self.after.as_deref())?,
        ))
    }
}

#[derive(Args)]
pub(super) struct Overview {
    owner: String,
    project: u32,
}

impl Overview {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.overview(&self.owner, self.project)?)
    }
}

#[derive(Args)]
pub(super) struct Structure {
    #[arg(long = "structure-file", value_name = "FILE")]
    structure: PathBuf,
}

impl Structure {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.structure(&decode(self.structure)?)?)
    }
}

pub(super) fn decode<T: DeserializeOwned>(file: PathBuf) -> Result<T, Error> {
    let input = read(file)?;
    if input.len() > 65_536 {
        return Err(Error::local(Cause::Input));
    }
    serde_json::from_str(&input).map_err(|_| Error::local(Cause::Input))
}

#[derive(Args)]
pub(super) struct Schema {
    #[arg(long = "schema-file", value_name = "FILE")]
    schema: PathBuf,
}

impl Schema {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        encode(client.schema(&decode(self.schema)?)?)
    }
}
