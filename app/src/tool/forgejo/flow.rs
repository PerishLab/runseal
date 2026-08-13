use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::Seat;

impl Seat {
    pub(super) fn standing(&self, id: &str) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "GET",
            &format!(
                "{}/repos/{owner}/{name}/commits/{}/status",
                self.base,
                super::quote(id)
            ),
            None,
        )
    }

    pub(super) fn sent(&self, id: &str) -> Result<Value> {
        let Some(git) = self.flag("ref").filter(|held| !held.is_empty()) else {
            bail!("@forgejo workflow dispatch requires --ref");
        };
        let inputs = match self.flag("inputs") {
            Some(raw) => serde_json::from_str(raw).context("invalid --inputs")?,
            None => json!({}),
        };
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "POST",
            &format!(
                "{}/repos/{owner}/{name}/actions/workflows/{}/dispatches",
                self.base,
                super::quote(id)
            ),
            Some(&json!({
                "ref": git,
                "inputs": inputs,
                "return_run_info": true
            })),
        )
    }

    pub(super) fn running(&self, id: &str) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "GET",
            &format!("{}/repos/{owner}/{name}/actions/runs/{id}", self.base),
            None,
        )
    }

    pub(super) fn logged(&self, run: &str, job: &str) -> Result<Value> {
        let attempt = self.flag("attempt").unwrap_or("1");
        let (owner, name) = super::pair(&self.repo()?)?;
        let origin = self.base.trim_end_matches("/api/v1");
        self.send(
            "GET",
            &format!(
                "{origin}/{owner}/{name}/actions/runs/{run}/jobs/{job}/attempt/{attempt}/logs"
            ),
            None,
        )
    }
}
