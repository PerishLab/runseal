use super::{Call, Cause, Client, Error, Pull, Receipt, Target, confirmed};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct Edit {
    title: String,
    body: String,
}

impl Edit {
    pub fn new(title: &str, body: &str) -> Result<Self, Error> {
        if title.trim().is_empty() || title.chars().count() > 256 || body.len() > 65_536 {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            title: title.into(),
            body: body.into(),
        })
    }

    fn confirmed(&self, found: &Pull) -> bool {
        found.title == self.title && found.body.as_deref().unwrap_or_default() == self.body
    }
}

impl Client {
    pub fn edit(&self, target: &Target, edit: &Edit) -> Result<Receipt<Pull>, Error> {
        let path = format!("/repos/{}/pulls/{}", target.name(), target.number());
        self.wire
            .send(&self.bearer, Call::Patch(&path, json!(edit)), |value| {
                let found: Pull = serde_json::from_value(value).ok()?;
                (confirmed(target, &found) && edit.confirmed(&found)).then_some(found)
            })
    }
}
