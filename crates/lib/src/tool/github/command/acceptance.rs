use super::delivery::encode;
use crate::github::{Client, Error, Target, Window, acceptance::Pair};
use clap::{Args, Subcommand};
use serde_json::Value;

#[derive(Args)]
pub(super) struct Acceptance {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    History(Selection),
    Comment {
        issue: String,
        comment: String,
    },
    Freshness {
        #[command(flatten)]
        scope: Selection,
        declaration: String,
        closure: String,
    },
}

#[derive(Args)]
struct Selection {
    owner: String,
    name: String,
    number: u32,
    #[arg(long, default_value_t = 30)]
    size: u8,
    #[arg(long)]
    after: Option<String>,
}

impl Selection {
    fn locate(&self) -> Result<(Target, Window), Error> {
        Ok((
            Target::new(&self.owner, &self.name, self.number)?,
            Window::new(self.size, self.after.as_deref())?,
        ))
    }
}

impl Acceptance {
    pub(super) fn execute(self, client: &Client) -> Result<Value, Error> {
        match self.action {
            Action::History(scope) => {
                let (target, window) = scope.locate()?;
                encode(client.reopenings(&target, &window)?)
            }
            Action::Comment { issue, comment } => encode(client.review(&issue, &comment)?),
            Action::Freshness {
                scope,
                declaration,
                closure,
            } => {
                let (target, window) = scope.locate()?;
                let pair = Pair::new(&declaration, &closure)?;
                encode(client.freshness(&target, &window, &pair)?)
            }
        }
    }
}
