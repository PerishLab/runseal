use super::{Failure, input, names::Names};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone)]
pub struct Script {
    pub instance: String,
    pub name: String,
    pub user: String,
    pub directory: String,
    pub timeout: u64,
    pub content: String,
}

impl fmt::Debug for Script {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Script")
            .field("instance", &self.instance)
            .field("content", &"withheld")
            .finish_non_exhaustive()
    }
}

impl Script {
    pub fn validate(&self) -> Result<(), Failure> {
        self.body().map(|_| ())
    }

    pub(super) fn body(&self) -> Result<Value, Failure> {
        Names::instance(&self.instance)?;
        if !Names::lexical(&self.name)
            || self.name.len() > 60
            || !(1..=86400).contains(&self.timeout)
        {
            return Err(input());
        }
        Self::user(&self.user)?;
        Self::directory(&self.directory)?;
        if self.content.is_empty() || self.content.len() > 49152 || self.content.contains('\0') {
            return Err(input());
        }
        Ok(
            json!({"InstanceIds":[self.instance],"Content":STANDARD.encode(self.content.as_bytes()),
            "CommandName":self.name,"CommandType":"SHELL","Username":self.user,
            "WorkingDirectory":self.directory,"Timeout":self.timeout,
            "SaveCommand":false,"EnableParameter":false}),
        )
    }

    pub(super) fn user(value: &str) -> Result<(), Failure> {
        let Some(first) = value.bytes().next() else {
            return Err(input());
        };
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Err(input());
        }
        if value.len() > 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
        {
            return Err(input());
        }
        Ok(())
    }

    pub(super) fn directory(value: &str) -> Result<(), Failure> {
        if !value.starts_with('/') || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(input());
        }
        Ok(())
    }
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
