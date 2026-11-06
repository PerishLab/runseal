use super::{Call, Error, identifier, input};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: Vec<Vec<Node>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "tag", deny_unknown_fields)]
pub enum Node {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "a")]
    Link { text: String, href: String },
}

#[derive(Clone, Debug)]
pub struct Post {
    pub message: String,
    pub document: Document,
    pub uuid: String,
    pub thread: bool,
}

impl Document {
    pub(super) fn encode(&self) -> Result<String, Error> {
        if self.content.is_empty() || self.content.len() > 64 {
            return Err(input());
        }
        if self
            .title
            .as_ref()
            .is_some_and(|title| title.len() > 256 || title.contains('\0'))
        {
            return Err(input());
        }
        let nodes = self.content.iter().map(Vec::len).sum::<usize>();
        if nodes == 0 || nodes > 512 || self.content.iter().any(Vec::is_empty) {
            return Err(input());
        }
        for node in self.content.iter().flatten() {
            node.validate()?;
        }
        let encoded = serde_json::to_string(&json!({"zh_cn":self})).map_err(|_| input())?;
        if encoded.len() > 30_000 {
            return Err(input());
        }
        Ok(encoded)
    }
}

impl Node {
    fn validate(&self) -> Result<(), Error> {
        let text = match self {
            Self::Text { text } | Self::Link { text, .. } => text,
        };
        if text.is_empty() || text.len() > 20_000 || text.contains('\0') {
            return Err(input());
        }
        if let Self::Link { href, .. } = self {
            let url = url::Url::parse(href).map_err(|_| input())?;
            if href.len() > 4_000
                || !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
            {
                return Err(input());
            }
            if !url.username().is_empty() || url.password().is_some() {
                return Err(input());
            }
        }
        Ok(())
    }
}

impl Post {
    pub(super) fn call(&self) -> Result<Call, Error> {
        identifier(&self.message, "om_")?;
        super::visible(&self.uuid, 50)?;
        Ok(Call {
            method: "POST",
            path: format!("im/v1/messages/{}/reply", self.message),
            body: Some(json!({"msg_type":"post", "content":self.document.encode()?,
                "uuid":self.uuid, "reply_in_thread":self.thread})),
            action: "im.messages.reply",
        })
    }
}
