use super::{Call, Error, Message, identifier, input, response, visible};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Page {
    pub chat: String,
    pub size: u8,
    pub cursor: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub ascending: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Messages {
    pub chat: String,
    pub items: Vec<Message>,
    pub more: bool,
    pub cursor: Option<String>,
}

impl Page {
    pub fn validate(&self) -> Result<(), Error> {
        self.call().map(|_| ())
    }

    pub(in super::super) fn call(&self) -> Result<Call, Error> {
        identifier(&self.chat, "oc_")?;
        if !(1..=50).contains(&self.size) {
            return Err(input());
        }
        if self.start.is_some_and(|n| n < 0)
            || self.end.is_some_and(|n| n < 0)
            || self.start.zip(self.end).is_some_and(|(s, e)| s >= e)
        {
            return Err(input());
        }
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query
            .append_pair("container_id_type", "chat")
            .append_pair("container_id", &self.chat)
            .append_pair("page_size", &self.size.to_string())
            .append_pair(
                "sort_type",
                if self.ascending {
                    "ByCreateTimeAsc"
                } else {
                    "ByCreateTimeDesc"
                },
            );
        if let Some(cursor) = &self.cursor {
            visible(cursor, 2048)?;
            query.append_pair("page_token", cursor);
        }
        if let Some(start) = self.start {
            query.append_pair("start_time", &start.to_string());
        }
        if let Some(end) = self.end {
            query.append_pair("end_time", &end.to_string());
        }
        Ok(Call {
            method: "GET",
            path: format!("im/v1/messages?{}", query.finish()),
            body: None,
            action: "im.messages.list",
        })
    }

    fn progress(&self, cursor: &Option<String>, empty: bool) -> Result<(), Error> {
        if cursor.is_none() || cursor == &self.cursor || empty {
            return Err(response());
        }
        Ok(())
    }

    pub(in super::super) fn decode(&self, value: &Value) -> Result<Messages, Error> {
        let value = &value["data"];
        let items: Vec<Message> =
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
            if item.chat != self.chat || !ids.insert(&item.id) {
                return Err(response());
            }
        }
        Ok(Messages {
            chat: self.chat.clone(),
            items,
            more,
            cursor,
        })
    }
}
