use super::{Failure, names::Names, response};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) struct Record<'a>(pub &'a Value);

impl<'a> Record<'a> {
    pub fn known(&self, fields: &[&str]) -> bool {
        self.0
            .as_object()
            .is_some_and(|object| object.keys().all(|key| fields.contains(&key.as_str())))
    }

    pub fn text(&self, key: &str, limit: usize) -> Result<Option<&'a str>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value))
                if value.len() <= limit && !value.chars().any(char::is_control) =>
            {
                Ok(Some(value))
            }
            _ => Err(response()),
        }
    }

    pub fn id(&self, key: &str, prefix: &str) -> Result<String, Failure> {
        let value = self.text(key, 64)?.ok_or_else(response)?;
        Names::id(value, prefix).map_err(|_| response())?;
        Ok(value.into())
    }

    pub fn array(&self, key: &str, limit: usize) -> Result<Option<&'a Vec<Value>>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(values)) if values.len() <= limit => Ok(Some(values)),
            _ => Err(response()),
        }
    }

    pub fn ids(&self, key: &str, prefix: &str) -> Result<Option<Vec<String>>, Failure> {
        let Some(values) = self.array(key, 1000)? else {
            return Ok(None);
        };
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for value in values {
            let id = value.as_str().ok_or_else(response)?;
            Names::id(id, prefix).map_err(|_| response())?;
            if !seen.insert(id) {
                return Err(response());
            }
            result.push(id.into());
        }
        Ok(Some(result))
    }

    pub fn nested(&self, key: &str) -> Result<Option<Record<'a>>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) if value.is_object() => Ok(Some(Record(value))),
            _ => Err(response()),
        }
    }

    pub fn time(&self, key: &str) -> Result<Option<String>, Failure> {
        let Some(value) = self.text(key, 64)? else {
            return Ok(None);
        };
        if !value.is_empty() {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                .map_err(|_| response())?;
        }
        Ok(Some(value.into()))
    }

    pub fn number(&self, key: &str) -> Result<u64, Failure> {
        self.0[key].as_u64().ok_or_else(response)
    }
}
