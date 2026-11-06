use super::*;
use crate::custody::Private;
use std::path::{Path, PathBuf};

impl Messages {
    pub(super) fn operation(&self) -> Result<Operation, Error> {
        match self {
            Self::Send {
                chat,
                recipient,
                text,
                uuid,
            } => {
                let target = match (chat, recipient) {
                    (Some(chat), None) => super::super::Target::Chat(chat.clone()),
                    (None, Some(user)) => super::super::Target::User(user.clone()),
                    _ => return Err(invalid()),
                };
                Ok(Operation::Send(super::super::Send {
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
                post,
                uuid,
                thread,
            } => {
                let body = body(text, post)?;
                match body {
                    super::super::Body::Text(text) => Ok(Operation::Reply(super::super::Reply {
                        message: message.clone(),
                        text,
                        uuid: uuid.clone(),
                        thread: *thread,
                    })),
                    super::super::Body::Post(document) => Ok(Operation::Post(super::super::Post {
                        message: message.clone(),
                        document,
                        uuid: uuid.clone(),
                        thread: *thread,
                    })),
                }
            }
            Self::Edit {
                message,
                text,
                post,
            } => Ok(Operation::Edit(super::super::Edit {
                message: message.clone(),
                body: body(text, post)?,
            })),
        }
    }
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

fn body(text: &Option<PathBuf>, post: &Option<PathBuf>) -> Result<super::super::Body, Error> {
    match (text, post) {
        (Some(path), None) => Ok(super::super::Body::Text(content(path)?)),
        (None, Some(path)) => {
            let bytes = Private::current()
                .read(path, 30_000)
                .map_err(|_| invalid())?;
            let document = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
            Ok(super::super::Body::Post(document))
        }
        _ => Err(invalid()),
    }
}
