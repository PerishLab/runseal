mod page;
mod send;
pub use page::{Messages, Page};
pub use send::{Send, Target};

use super::{
    Error,
    fault::{input, response},
    model::{identifier, visible},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub enum Operation {
    Get { message: String },
    Reply(Reply),
    Send(Send),
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub message: String,
    pub text: String,
    pub uuid: String,
    pub thread: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sender {
    pub id: String,
    #[serde(rename = "id_type")]
    pub scheme: String,
    #[serde(rename = "sender_type")]
    pub kind: String,
    #[serde(rename = "tenant_key")]
    pub tenant: Option<String>,
    #[serde(rename = "open_bot_id")]
    pub bot: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    #[serde(rename = "message_id")]
    pub id: String,
    #[serde(rename = "chat_id")]
    pub chat: String,
    #[serde(rename = "msg_type")]
    pub kind: String,
    pub sender: Sender,
    pub body: Content,
    #[serde(rename = "parent_id")]
    pub parent: Option<String>,
    #[serde(rename = "root_id")]
    pub root: Option<String>,
    #[serde(rename = "thread_id")]
    pub thread: Option<String>,
    #[serde(rename = "create_time")]
    pub created: Option<String>,
    #[serde(rename = "update_time")]
    pub modified: Option<String>,
    pub deleted: Option<bool>,
    pub updated: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Content {
    pub content: String,
}

pub(super) struct Call {
    pub method: &'static str,
    pub path: String,
    pub body: Option<Value>,
    pub action: &'static str,
}

impl Operation {
    pub fn validate(&self) -> Result<(), Error> {
        self.call().map(|_| ())
    }

    pub(super) fn call(&self) -> Result<Call, Error> {
        match self {
            Self::Get { message } => {
                identifier(message, "om_")?;
                Ok(Call {
                    method: "GET",
                    path: format!("im/v1/messages/{message}?user_id_type=open_id"),
                    body: None,
                    action: "im.messages.get",
                })
            }
            Self::Reply(reply) => reply.call(),
            Self::Send(send) => send.call(),
        }
    }

    pub(super) fn decode(&self, value: &Value) -> Result<Message, Error> {
        let record = match self {
            Self::Get { .. } => {
                let items = value["data"]["items"].as_array().ok_or_else(response)?;
                if items.len() != 1 {
                    return Err(response());
                }
                &items[0]
            }
            Self::Reply(_) | Self::Send(_) => &value["data"],
        };
        let message: Message = serde_json::from_value(record.clone()).map_err(|_| response())?;
        message.validate()?;
        match self {
            Self::Get { message: expected } if &message.id != expected => Err(response()),
            Self::Reply(reply) if message.parent.as_deref() != Some(&reply.message) => {
                Err(response())
            }
            Self::Send(send) if !send.matches(&message) => Err(response()),
            _ => Ok(message),
        }
    }
}

impl Reply {
    fn call(&self) -> Result<Call, Error> {
        identifier(&self.message, "om_")?;
        visible(&self.uuid, 50)?;
        if self.text.trim().is_empty() || self.text.len() > 20_000 || self.text.contains('\0') {
            return Err(input());
        }
        Ok(Call {
            method: "POST",
            path: format!("im/v1/messages/{}/reply", self.message),
            body: Some(json!({
                "msg_type":"text", "content":json!({"text":self.text}).to_string(),
                "uuid":self.uuid, "reply_in_thread":self.thread,
            })),
            action: "im.messages.reply",
        })
    }
}

impl Message {
    pub(super) fn validate(&self) -> Result<(), Error> {
        identifier(&self.id, "om_").map_err(|_| response())?;
        identifier(&self.chat, "oc_").map_err(|_| response())?;
        visible(&self.kind, 64).map_err(|_| response())?;
        self.sender.validate()?;
        if self.body.content.len() > 512 * 1024 {
            return Err(response());
        }
        Ok(())
    }
}

impl Sender {
    fn validate(&self) -> Result<(), Error> {
        let prefix = match (self.kind.as_str(), self.scheme.as_str()) {
            ("user" | "app", "open_id") => "ou_",
            ("app", "app_id") => "cli_",
            _ => return Err(response()),
        };
        identifier(&self.id, prefix).map_err(|_| response())
    }

    pub(super) fn matches(&self, identity: &super::Identity) -> bool {
        if let Some(user) = &identity.user {
            return self.kind == "user" && self.scheme == "open_id" && &self.id == user;
        }
        self.kind == "app"
            && match self.scheme.as_str() {
                "app_id" => self.id == identity.app,
                "open_id" => self.id == identity.bot,
                _ => false,
            }
    }
}

#[cfg(test)]
#[path = "../tests/message.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/page.rs"]
mod pages;
