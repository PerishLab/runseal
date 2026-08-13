use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{Call, pair};
use crate::core::wire;

impl Call {
    pub(crate) fn shown(&self, id: &str, token: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        wire::get(
            &format!("{}/repos/{owner}/{name}/issues/{index}", self.base),
            token,
        )
    }

    pub(crate) fn listed(&self, token: &str) -> Result<Value> {
        let (owner, name) = pair(&self.repo()?)?;
        self.pages(
            token,
            &format!(
                "{}/repos/{owner}/{name}/issues?state=all&limit=50",
                self.base
            ),
        )
    }

    pub(crate) fn created(&self, token: &str) -> Result<Value> {
        let Some(title) = self.title.as_deref().filter(|held| !held.is_empty()) else {
            bail!("@forgejo issue create requires --title");
        };
        let (owner, name) = pair(&self.repo()?)?;
        let mut body = json!({ "title": title });
        if let Some(text) = &self.body {
            body["body"] = json!(text);
        }
        wire::post(
            &format!("{}/repos/{owner}/{name}/issues", self.base),
            token,
            &body,
        )
    }

    pub(crate) fn edited(&self, id: &str, token: &str) -> Result<Value> {
        let mut body = serde_json::Map::new();
        if let Some(title) = &self.title {
            body.insert("title".into(), json!(title));
        }
        if let Some(text) = &self.body {
            body.insert("body".into(), json!(text));
        }
        if let Some(state) = &self.state {
            if state != "open" && state != "closed" {
                bail!("@forgejo issue edit --state must be open or closed");
            }
            body.insert("state".into(), json!(state));
        }
        if body.is_empty() {
            bail!("@forgejo issue edit requires --title, --body, or --state");
        }
        let (owner, name, index) = self.target(id)?;
        wire::patch(
            &format!("{}/repos/{owner}/{name}/issues/{index}", self.base),
            token,
            &Value::Object(body),
        )
    }

    pub(crate) fn notes(&self, id: &str, token: &str) -> Result<Value> {
        let (owner, name, index) = self.target(id)?;
        self.pages(
            token,
            &format!(
                "{}/repos/{owner}/{name}/issues/{index}/comments?limit=50",
                self.base
            ),
        )
    }

    pub(crate) fn posted(&self, id: &str, token: &str) -> Result<Value> {
        let Some(text) = self.body.as_deref().filter(|held| !held.is_empty()) else {
            bail!("@forgejo issue comment create requires --body");
        };
        let (owner, name, index) = self.target(id)?;
        wire::post(
            &format!("{}/repos/{owner}/{name}/issues/{index}/comments", self.base),
            token,
            &json!({ "body": text }),
        )
    }

    fn pages(&self, token: &str, stem: &str) -> Result<Value> {
        let mut items = Vec::new();
        let mut turn = 1usize;
        loop {
            let value = wire::get(&format!("{stem}&page={turn}"), token)?;
            let Some(rows) = value.as_array() else {
                bail!("forgejo list did not return an array");
            };
            let count = rows.len();
            items.extend(rows.iter().cloned());
            if count < 50 || self.limit.is_some_and(|limit| items.len() >= limit) {
                break;
            }
            turn += 1;
        }
        Ok(Value::Array(items))
    }
}
