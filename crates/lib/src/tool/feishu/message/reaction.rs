use super::super::{
    Error, Identity,
    fault::{input, response},
    message::Call,
    model::{identifier, visible},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Create {
    pub message: String,
    pub emoji: String,
}

#[derive(Clone, Debug)]
pub struct Page {
    pub message: String,
    pub emoji: Option<String>,
    pub size: u8,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Operator {
    #[serde(rename = "operator_id")]
    pub id: String,
    #[serde(rename = "operator_type")]
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Emoji {
    #[serde(rename = "emoji_type")]
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reaction {
    #[serde(rename = "reaction_id")]
    pub id: String,
    pub operator: Operator,
    #[serde(rename = "action_time")]
    pub created: String,
    #[serde(rename = "reaction_type")]
    pub emoji: Emoji,
}

#[derive(Clone, Debug, Serialize)]
pub struct Created {
    pub message: String,
    pub reaction: Reaction,
}

#[derive(Clone, Debug, Serialize)]
pub struct Reactions {
    pub message: String,
    pub items: Vec<Reaction>,
    pub more: bool,
    pub cursor: Option<String>,
}

impl Create {
    pub fn validate(&self) -> Result<(), Error> {
        self.call().map(|_| ())
    }

    pub(in super::super) fn call(&self) -> Result<Call, Error> {
        identifier(&self.message, "om_")?;
        emoji(&self.emoji)?;
        Ok(Call {
            method: "POST",
            path: format!("im/v1/messages/{}/reactions", self.message),
            body: Some(json!({"reaction_type":{"emoji_type":self.emoji}})),
            action: "im.reactions.create",
        })
    }

    pub(in super::super) fn decode(
        &self,
        value: &Value,
        identity: &Identity,
    ) -> Result<Created, Error> {
        let reaction: Reaction =
            serde_json::from_value(value["data"].clone()).map_err(|_| response())?;
        reaction.validate()?;
        if reaction.emoji.kind != self.emoji || !reaction.operator.matches(identity) {
            return Err(response());
        }
        Ok(Created {
            message: self.message.clone(),
            reaction,
        })
    }
}

impl Page {
    pub fn validate(&self) -> Result<(), Error> {
        self.call().map(|_| ())
    }

    pub(in super::super) fn call(&self) -> Result<Call, Error> {
        identifier(&self.message, "om_")?;
        if !(1..=50).contains(&self.size) {
            return Err(input());
        }
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query
            .append_pair("user_id_type", "open_id")
            .append_pair("page_size", &self.size.to_string());
        if let Some(kind) = &self.emoji {
            emoji(kind)?;
            query.append_pair("reaction_type", kind);
        }
        if let Some(cursor) = &self.cursor {
            visible(cursor, 2048)?;
            query.append_pair("page_token", cursor);
        }
        Ok(Call {
            method: "GET",
            path: format!(
                "im/v1/messages/{}/reactions?{}",
                self.message,
                query.finish()
            ),
            body: None,
            action: "im.reactions.list",
        })
    }

    fn progress(&self, cursor: &Option<String>, empty: bool) -> Result<(), Error> {
        if empty || cursor.is_none() || cursor == &self.cursor {
            return Err(response());
        }
        Ok(())
    }

    pub(in super::super) fn decode(&self, value: &Value) -> Result<Reactions, Error> {
        let value = &value["data"];
        let items: Vec<Reaction> =
            serde_json::from_value(value["items"].clone()).map_err(|_| response())?;
        let more = value["has_more"].as_bool().ok_or_else(response)?;
        let cursor = match &value["page_token"] {
            Value::Null => None,
            Value::String(s) if s.is_empty() => None,
            Value::String(s) => {
                visible(s, 2048).map_err(|_| response())?;
                Some(s.clone())
            }
            _ => return Err(response()),
        };
        if items.len() > usize::from(self.size) {
            return Err(response());
        }
        if more {
            self.progress(&cursor, items.is_empty())?;
        }
        let mut ids = BTreeSet::new();
        for item in &items {
            item.validate()?;
            if !ids.insert(&item.id)
                || self
                    .emoji
                    .as_ref()
                    .is_some_and(|kind| &item.emoji.kind != kind)
            {
                return Err(response());
            }
        }
        Ok(Reactions {
            message: self.message.clone(),
            items,
            more,
            cursor,
        })
    }
}

impl Reaction {
    fn validate(&self) -> Result<(), Error> {
        visible(&self.id, 512).map_err(|_| response())?;
        emoji(&self.emoji.kind).map_err(|_| response())?;
        if self
            .created
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .is_none()
        {
            return Err(response());
        }
        let prefix = match self.operator.kind.as_str() {
            "app" => "cli_",
            "user" => "ou_",
            _ => return Err(response()),
        };
        identifier(&self.operator.id, prefix).map_err(|_| response())
    }
}

impl Operator {
    fn matches(&self, identity: &Identity) -> bool {
        match &identity.user {
            Some(user) => self.kind == "user" && &self.id == user,
            None => self.kind == "app" && self.id == identity.app,
        }
    }
}

fn emoji(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(input());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/reaction.rs"]
mod tests;
