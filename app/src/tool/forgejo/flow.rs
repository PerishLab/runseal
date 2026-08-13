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

    pub(super) fn tasks(&self, run: &str) -> Result<Value> {
        let run = run.parse::<u64>().context("invalid task run number")?;
        let (owner, name) = super::pair(&self.repo()?)?;
        let mut page = 1;
        let mut seen = 0;
        let mut found = Vec::new();
        loop {
            let value = self.send(
                "GET",
                &format!(
                    "{}/repos/{owner}/{name}/actions/tasks?page={page}&limit=50",
                    self.base
                ),
                None,
            )?;
            let rows = value
                .get("workflow_runs")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow::anyhow!("Forgejo action task list has no workflow_runs"))?;
            let total = value
                .get("total_count")
                .and_then(Value::as_u64)
                .unwrap_or(rows.len() as u64) as usize;
            seen += rows.len();
            found.extend(
                rows.iter()
                    .filter(|task| task.get("run_number").and_then(Value::as_u64) == Some(run))
                    .cloned(),
            );
            if seen >= total || rows.len() < 50 {
                break;
            }
            page += 1;
        }
        Ok(Value::Array(found))
    }
}
