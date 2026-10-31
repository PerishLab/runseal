use super::{Call, Error, Message, identifier, input, visible};
use serde_json::json;

#[derive(Clone, Debug)]
pub enum Target {
    Chat(String),
    User(String),
}

#[derive(Clone, Debug)]
pub struct Send {
    pub target: Target,
    pub text: String,
    pub uuid: String,
}

impl Send {
    pub(super) fn call(&self) -> Result<Call, Error> {
        let (scheme, target) = match &self.target {
            Target::Chat(chat) => {
                identifier(chat, "oc_")?;
                ("chat_id", chat)
            }
            Target::User(user) => {
                identifier(user, "ou_")?;
                ("open_id", user)
            }
        };
        visible(&self.uuid, 50)?;
        if self.text.trim().is_empty() || self.text.len() > 20_000 || self.text.contains('\0') {
            return Err(input());
        }
        Ok(Call {
            method: "POST",
            path: format!("im/v1/messages?receive_id_type={scheme}"),
            body: Some(json!({"receive_id":target,"msg_type":"text",
                "content":json!({"text":self.text}).to_string(),"uuid":self.uuid})),
            action: "im.messages.send",
        })
    }

    pub(super) fn matches(&self, message: &Message) -> bool {
        match &self.target {
            Target::Chat(chat) => chat == &message.chat,
            Target::User(_) => true,
        }
    }
}
