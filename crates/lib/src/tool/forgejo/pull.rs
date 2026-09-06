use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::Seat;

impl Seat {
    pub(super) fn opened(&self, id: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        self.send(
            "GET",
            &format!("{}/repos/{owner}/{name}/pulls/{index}", self.base),
            None,
        )
    }

    pub(super) fn pulls(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.pages(&format!(
            "{}/repos/{owner}/{name}/pulls?state=all&limit=50",
            self.base
        ))
    }

    pub(super) fn raised(&self) -> Result<Value> {
        let Some(title) = self.flag("title").filter(|held| !held.is_empty()) else {
            bail!("@forgejo pull create requires --title");
        };
        let Some(head) = self.flag("head").filter(|held| !held.is_empty()) else {
            bail!("@forgejo pull create requires --head");
        };
        let onto = self.flag("base").unwrap_or("main");
        let (owner, name) = super::pair(&self.repo()?)?;
        let mut body = json!({ "title": title, "head": head, "base": onto });
        if let Some(text) = self.flag("body") {
            body["body"] = json!(text);
        }
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/pulls", self.base),
            Some(&body),
        )
    }

    pub(super) fn amended(&self, id: &str) -> Result<Value> {
        let mut body = serde_json::Map::new();
        if let Some(title) = self.flag("title") {
            body.insert("title".into(), json!(title));
        }
        if let Some(text) = self.flag("body") {
            body.insert("body".into(), json!(text));
        }
        if let Some(state) = self.flag("state") {
            body.insert("state".into(), json!(state));
        }
        if body.is_empty() {
            bail!("@forgejo pull edit requires --title, --body, or --state");
        }
        let (owner, name, index) = self.target(id)?;
        self.send(
            "PATCH",
            &format!("{}/repos/{owner}/{name}/pulls/{index}", self.base),
            Some(&Value::Object(body)),
        )
    }

    pub(super) fn merged(&self, id: &str) -> Result<Value> {
        let Some(head) = self.flag("head").filter(|held| !held.is_empty()) else {
            bail!("@forgejo pull merge requires --head");
        };
        let way = self.flag("do").unwrap_or("fast-forward-only");
        let (owner, name, index) = self.target(id)?;
        let body = json!({
            "Do": way,
            "delete_branch_after_merge": false,
            "head_commit_id": head
        });
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/pulls/{index}/merge", self.base),
            Some(&body),
        )
    }

    pub(super) fn reviews(&self, id: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        self.pages(&format!(
            "{}/repos/{owner}/{name}/pulls/{index}/reviews?limit=50",
            self.base
        ))
    }

    pub(super) fn reviewed(&self, id: &str) -> Result<Value> {
        let Some(text) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo review create requires --body");
        };
        let event = self.flag("event").unwrap_or("COMMENT");
        let (owner, name, index) = self.target(id)?;
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/pulls/{index}/reviews", self.base),
            Some(&json!({ "body": text, "event": event })),
        )
    }
}
