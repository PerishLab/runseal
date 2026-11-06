use super::{Call, Document, Error, identifier, input};
use serde_json::json;

#[derive(Clone, Debug)]
pub enum Body {
    Text(String),
    Post(Document),
}

#[derive(Clone, Debug)]
pub struct Edit {
    pub message: String,
    pub body: Body,
}

impl Body {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Post(_) => "post",
        }
    }

    fn encode(&self) -> Result<String, Error> {
        match self {
            Self::Post(document) => document.encode(),
            Self::Text(text) => {
                if text.trim().is_empty() || text.len() > 20_000 || text.contains('\0') {
                    return Err(input());
                }
                Ok(json!({"text":text}).to_string())
            }
        }
    }
}

impl Edit {
    pub(super) fn call(&self) -> Result<Call, Error> {
        identifier(&self.message, "om_")?;
        Ok(Call {
            method: "PUT",
            path: format!("im/v1/messages/{}", self.message),
            body: Some(json!({"msg_type":self.body.kind(), "content":self.body.encode()?})),
            action: "im.messages.edit",
        })
    }
}
