use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::Seat;

impl Seat {
    pub(super) fn shown(&self, id: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        self.send(
            "GET",
            &format!("{}/repos/{owner}/{name}/issues/{index}", self.base),
            None,
        )
    }

    pub(super) fn listed(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.pages(&format!(
            "{}/repos/{owner}/{name}/issues?state=all&limit=50",
            self.base
        ))
    }

    pub(super) fn created(&self) -> Result<Value> {
        let Some(title) = self.flag("title").filter(|held| !held.is_empty()) else {
            bail!("@forgejo issue create requires --title");
        };
        let (owner, name) = super::pair(&self.repo()?)?;
        let mut body = json!({ "title": title });
        if let Some(text) = self.flag("body") {
            body["body"] = json!(text);
        }
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/issues", self.base),
            Some(&body),
        )
    }

    pub(super) fn edited(&self, id: &str) -> Result<Value> {
        let mut body = serde_json::Map::new();
        if let Some(title) = self.flag("title") {
            body.insert("title".into(), json!(title));
        }
        if let Some(text) = self.flag("body") {
            body.insert("body".into(), json!(text));
        }
        if let Some(state) = self.flag("state") {
            if state != "open" && state != "closed" {
                bail!("@forgejo issue edit --state must be open or closed");
            }
            body.insert("state".into(), json!(state));
        }
        if body.is_empty() {
            bail!("@forgejo issue edit requires --title, --body, or --state");
        }
        let (owner, name, index) = self.target(id)?;
        self.send(
            "PATCH",
            &format!("{}/repos/{owner}/{name}/issues/{index}", self.base),
            Some(&Value::Object(body)),
        )
    }

    pub(super) fn notes(&self, id: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        self.pages(&format!(
            "{}/repos/{owner}/{name}/issues/{index}/comments?limit=50",
            self.base
        ))
    }

    pub(super) fn posted(&self, id: &str) -> Result<Value> {
        let Some(text) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo issue comment create requires --body");
        };
        let (owner, name, index) = self.target(id)?;
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/issues/{index}/comments", self.base),
            Some(&json!({ "body": text })),
        )
    }
}
