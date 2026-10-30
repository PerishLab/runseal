use super::{Class, Client, Error, Operation, Reply, fault::input as invalid};
use crate::custody::Private;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "@feishu",
    about = "Native Feishu operations; stage 1 custom-app bot identity and atomic messages. No automatic login, renewal, retries, enrichment or pagination."
)]
pub(super) struct Input {
    #[arg(
        long,
        global = true,
        help = "Absolute private state file; otherwise profile FEISHU_STATE_FILE"
    )]
    state: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Expected App ID; otherwise profile FEISHU_APP_ID"
    )]
    app: Option<String>,
    #[arg(long = "as", global = true, value_enum, default_value = "bot")]
    _actor: Actor,
    #[arg(
        long,
        global = true,
        help = "Structured output; failures also return nonzero"
    )]
    pub json: bool,
    #[command(subcommand)]
    deed: Deed,
}

#[derive(Clone, Copy, ValueEnum)]
enum Actor {
    Bot,
}

#[derive(Subcommand)]
enum Deed {
    #[command(about = "Explicit local custody, token issuance and bot identity verification")]
    Auth {
        #[command(subcommand)]
        deed: Auth,
    },
    #[command(about = "Instant messaging; no hidden name/reaction/thread/resource enrichment")]
    Im {
        #[command(subcommand)]
        deed: Im,
    },
}

#[derive(Subcommand)]
enum Auth {
    #[command(
        about = "Import private JSON {app_id,app_secret} into a new private state; no network call"
    )]
    Import {
        #[arg(long = "credentials-file")]
        credentials: PathBuf,
    },
    #[command(about = "Read nonsecret local state; no network call or implicit refresh")]
    Status,
    #[command(
        about = "Issue a Bearer tenant token with one client_credentials request; clears identity binding"
    )]
    Renew,
    #[command(
        about = "Verify app-local bot identity with one API call; binds the current token, not a person or resource permission"
    )]
    Identity,
}

#[derive(Subcommand)]
enum Im {
    Messages {
        #[command(subcommand)]
        deed: Messages,
    },
}

#[derive(Subcommand)]
enum Messages {
    #[command(about = "Read one exact message, with sender IDs as open_id; no enrichment")]
    Get {
        #[arg(long = "message-id")]
        message: String,
    },
    #[command(
        about = "Reply once with plain text; caller owns sender policy and durable intent. Accepted does not prove delivery. UUID dedup is time-bounded (one hour), not durable exactly-once."
    )]
    Reply {
        #[arg(long = "message-id")]
        message: String,
        #[arg(
            long = "text-file",
            help = "Absolute private UTF-8 file, 1..20000 bytes"
        )]
        text: PathBuf,
        #[arg(
            long = "idempotency-key",
            help = "Explicit UUID, 1..50 visible ASCII bytes; persist exact target/content/UUID before dispatch"
        )]
        uuid: String,
        #[arg(long = "reply-in-thread")]
        thread: bool,
    },
}

pub(super) fn execute(input: Input, vars: &BTreeMap<String, String>) -> Result<Value, Error> {
    let path = input
        .state
        .or_else(|| vars.get("FEISHU_STATE_FILE").map(Into::into))
        .ok_or_else(|| {
            Error::new(
                Class::Input,
                "@feishu requires an absolute --state or profile FEISHU_STATE_FILE",
            )
        })?;
    let app = input
        .app
        .or_else(|| vars.get("FEISHU_APP_ID").cloned())
        .ok_or_else(|| {
            Error::new(
                Class::Input,
                "@feishu requires an expected --app or profile FEISHU_APP_ID",
            )
        })?;
    if let Deed::Auth {
        deed: Auth::Import { credentials },
    } = &input.deed
    {
        return value(Client::import(&path, &app, credentials)?);
    }
    let operation = operation(&input.deed)?;
    let mut client = Client::open(&path, &app)?;
    if let Some(operation) = operation {
        return value(client.message(&operation)?);
    }
    match input.deed {
        Deed::Auth { deed: Auth::Status } => value(client.status()),
        Deed::Auth { deed: Auth::Renew } => value(client.renew()?),
        Deed::Auth {
            deed: Auth::Identity,
        } => value(client.identity()?),
        _ => Err(invalid()),
    }
}

fn operation(deed: &Deed) -> Result<Option<Operation>, Error> {
    match deed {
        Deed::Im {
            deed: Im::Messages { deed },
        } => {
            let operation = deed.operation()?;
            operation.validate()?;
            Ok(Some(operation))
        }
        _ => Ok(None),
    }
}

impl Messages {
    fn operation(&self) -> Result<Operation, Error> {
        match self {
            Self::Get { message } => Ok(Operation::Get {
                message: message.clone(),
            }),
            Self::Reply {
                message,
                text,
                uuid,
                thread,
            } => {
                let bytes = Private::current().read(text, 20_000).map_err(|_| {
                    Error::new(
                        Class::Input,
                        "Feishu reply needs an absolute private UTF-8 text file up to 20000 bytes",
                    )
                })?;
                Ok(Operation::Reply(Reply {
                    message: message.clone(),
                    text: String::from_utf8(bytes).map_err(|_| invalid())?,
                    uuid: uuid.clone(),
                    thread: *thread,
                }))
            }
        }
    }
}

fn value<T: Serialize>(value: T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| invalid())
}
