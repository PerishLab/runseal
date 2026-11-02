use super::{Call, Error, identifier, response, visible};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Get {
    pub chat: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Chat {
    pub requested: String,
    #[serde(flatten)]
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub name: Option<String>,
    #[serde(rename(deserialize = "chat_mode"))]
    pub mode: Option<String>,
    #[serde(rename(deserialize = "chat_type"))]
    pub kind: Option<String>,
    pub external: Option<bool>,
}

impl Get {
    pub fn validate(&self) -> Result<(), Error> {
        identifier(&self.chat, "oc_")
    }

    pub(in super::super) fn call(&self) -> Result<Call, Error> {
        self.validate()?;
        Ok(Call {
            method: "GET",
            path: format!("im/v1/chats/{}?user_id_type=open_id", self.chat),
            body: None,
            action: "im.chats.get",
        })
    }

    pub(in super::super) fn decode(&self, value: &Value) -> Result<Chat, Error> {
        let data = value["data"].as_object().ok_or_else(response)?;
        if let Some(echo) = data.get("chat_id")
            && echo.as_str() != Some(&self.chat)
        {
            return Err(response());
        }
        let metadata: Metadata =
            serde_json::from_value(value["data"].clone()).map_err(|_| response())?;
        if let Some(name) = &metadata.name
            && (name.len() > 2048 || name.chars().any(char::is_control))
        {
            return Err(response());
        }
        for label in [&metadata.mode, &metadata.kind].into_iter().flatten() {
            visible(label, 128).map_err(|_| response())?;
        }
        Ok(Chat {
            requested: self.chat.clone(),
            metadata,
        })
    }
}

#[cfg(test)]
#[path = "../tests/chat.rs"]
mod tests;
