use super::{Call, Error, identifier, input, response, visible};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Search {
    pub query: String,
    pub start: i64,
    pub end: i64,
    pub size: u8,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Matches {
    pub items: Vec<String>,
    pub more: bool,
    pub cursor: Option<String>,
}

impl Search {
    pub fn validate(&self) -> Result<(), Error> {
        self.call().map(|_| ())
    }

    pub(in super::super) fn call(&self) -> Result<Call, Error> {
        if self.query.len() > 4096
            || self.query.chars().any(char::is_control)
            || !(1..=50).contains(&self.size)
        {
            return Err(input());
        }
        if self.start < 0 || self.start >= self.end {
            return Err(input());
        }
        let start = timestamp(self.start)?;
        let end = timestamp(self.end)?;
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("page_size", &self.size.to_string());
        if let Some(cursor) = &self.cursor {
            visible(cursor, 2048)?;
            query.append_pair("page_token", cursor);
        }
        Ok(Call {
            method: "POST",
            path: format!("im/v1/messages/search?{}", query.finish()),
            body: Some(json!({"query":self.query,"filter":{"time_range":{
                "start_time":start,"end_time":end
            }}})),
            action: "im.messages.search",
        })
    }

    fn progress(&self, cursor: &Option<String>, empty: bool) -> Result<(), Error> {
        if cursor.is_none() || cursor == &self.cursor || empty {
            return Err(response());
        }
        Ok(())
    }

    pub(in super::super) fn decode(&self, value: &Value) -> Result<Matches, Error> {
        let value = &value["data"];
        let raw = value["items"].as_array().ok_or_else(response)?;
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
        if raw.len() > usize::from(self.size) {
            return Err(response());
        }
        if more {
            self.progress(&cursor, raw.is_empty())?;
        }
        let mut ids = BTreeSet::new();
        let mut items = Vec::new();
        for item in raw {
            let id = item["meta_data"]["message_id"]
                .as_str()
                .ok_or_else(response)?;
            identifier(id, "om_").map_err(|_| response())?;
            if !ids.insert(id) {
                return Err(response());
            }
            items.push(id.into());
        }
        Ok(Matches {
            items,
            more,
            cursor,
        })
    }
}

fn timestamp(seconds: i64) -> Result<String, Error> {
    let time = time::OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| input())?;
    if !(1970..=9999).contains(&time.year()) {
        return Err(input());
    }
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        time.year(),
        time.month() as u8,
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    ))
}

#[cfg(test)]
#[path = "../tests/search.rs"]
mod tests;
