use super::{Class, Client, Error, Operation, Reply, fault::input as invalid};
use crate::custody::Private;
mod args;
use super::Actor;
pub(super) use args::Input;
use args::{Auth, Deed, Im, Messages, Reactions};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

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
    validate(&input.deed)?;
    let mut client = match actor(input.actor, vars)? {
        Actor::Bot => {
            if input.user.is_some() {
                return Err(invalid());
            }
            Client::open(&path, &app)?
        }
        Actor::User => {
            let expected = input
                .user
                .as_ref()
                .or_else(|| vars.get("FEISHU_USER_OPEN_ID"))
                .ok_or_else(|| {
                    Error::new(
                        Class::Input,
                        "Feishu --as user requires --user or profile FEISHU_USER_OPEN_ID",
                    )
                })?;
            Client::user(&path, &app, expected)?
        }
    };
    if let Deed::Im {
        deed: Im::Reactions { deed },
    } = &input.deed
    {
        return match deed {
            Reactions::Create { message, emoji } => {
                value(client.reaction(&super::reaction::Create {
                    message: message.clone(),
                    emoji: emoji.clone(),
                })?)
            }
            Reactions::List {
                message,
                emoji,
                size,
                cursor,
            } => value(client.reactions(&super::reaction::Page {
                message: message.clone(),
                emoji: emoji.clone(),
                size: *size,
                cursor: cursor.clone(),
            })?),
        };
    }
    if let Deed::Im {
        deed:
            Im::Messages {
                deed:
                    Messages::List {
                        chat,
                        size,
                        cursor,
                        start,
                        end,
                        ascending,
                    },
            },
    } = &input.deed
    {
        return value(client.messages(&super::Page {
            chat: chat.clone(),
            size: *size,
            cursor: cursor.clone(),
            start: *start,
            end: *end,
            ascending: *ascending,
        })?);
    }
    if let Some(search) = search(&input.deed) {
        return value(client.search(&search)?);
    }
    if let Some(operation) = operation {
        return value(client.message(&operation)?);
    }
    match input.deed {
        Deed::Auth { deed: Auth::Status } => value(client.status()),
        Deed::Auth {
            deed: Auth::Begin { scopes },
        } => value(client.begin(&scopes)?),
        Deed::Auth {
            deed: Auth::Exchange,
        } => value(client.exchange()?),
        Deed::Auth {
            deed: Auth::Recover,
        } => value(client.recover()?),
        Deed::Auth { deed: Auth::Logout } => value(client.logout()?),
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
            if matches!(deed, Messages::List { .. } | Messages::Search { .. }) {
                return Ok(None);
            }
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
            Self::Send {
                chat,
                recipient,
                text,
                uuid,
            } => {
                let target = match (chat, recipient) {
                    (Some(chat), None) => super::Target::Chat(chat.clone()),
                    (None, Some(user)) => super::Target::User(user.clone()),
                    _ => return Err(invalid()),
                };
                Ok(Operation::Send(super::Send {
                    target,
                    text: content(text)?,
                    uuid: uuid.clone(),
                }))
            }
            Self::List { .. } | Self::Search { .. } => Err(invalid()),
            Self::Get { message } => Ok(Operation::Get {
                message: message.clone(),
            }),
            Self::Reply {
                message,
                text,
                uuid,
                thread,
            } => Ok(Operation::Reply(Reply {
                message: message.clone(),
                text: content(text)?,
                uuid: uuid.clone(),
                thread: *thread,
            })),
        }
    }
}

fn value<T: Serialize>(value: T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| invalid())
}

fn content(path: &Path) -> Result<String, Error> {
    let bytes = Private::current().read(path, 20_000).map_err(|_| {
        Error::new(
            Class::Input,
            "Feishu text needs an absolute private UTF-8 file up to 20000 bytes",
        )
    })?;
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn actor(input: Option<Actor>, vars: &BTreeMap<String, String>) -> Result<Actor, Error> {
    if let Some(actor) = input {
        return Ok(actor);
    }
    match vars.get("FEISHU_ACTOR").map(String::as_str) {
        None | Some("bot") => Ok(Actor::Bot),
        Some("user") => Ok(Actor::User),
        _ => Err(invalid()),
    }
}

fn search(deed: &Deed) -> Option<super::Search> {
    match deed {
        Deed::Im {
            deed:
                Im::Messages {
                    deed:
                        Messages::Search {
                            query,
                            start,
                            end,
                            size,
                            cursor,
                        },
                },
        } => Some(super::Search {
            query: query.clone(),
            start: *start,
            end: *end,
            size: *size,
            cursor: cursor.clone(),
        }),
        _ => None,
    }
}

fn validate(deed: &Deed) -> Result<(), Error> {
    if let Some(search) = search(deed) {
        return search.validate();
    }
    match deed {
        Deed::Im {
            deed:
                Im::Reactions {
                    deed: Reactions::Create { message, emoji },
                },
        } => super::reaction::Create {
            message: message.clone(),
            emoji: emoji.clone(),
        }
        .validate(),
        Deed::Im {
            deed:
                Im::Reactions {
                    deed:
                        Reactions::List {
                            message,
                            emoji,
                            size,
                            cursor,
                        },
                },
        } => super::reaction::Page {
            message: message.clone(),
            emoji: emoji.clone(),
            size: *size,
            cursor: cursor.clone(),
        }
        .validate(),
        _ => Ok(()),
    }
}
