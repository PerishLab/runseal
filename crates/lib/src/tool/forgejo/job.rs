use std::{
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::Seat;

impl Seat {
    pub(super) fn follow(&self, run: &str, job: &str) -> Result<()> {
        if self.line.json {
            bail!("@forgejo job log --watch does not support --json");
        }
        let poll = self.millis("poll-ms", 1_000)?;
        let deadline = Instant::now() + self.millis("timeout-ms", 30 * 60 * 1_000)?;
        let mut shown = String::new();
        loop {
            let (status, done) = self.state(run, job)?;
            if matches!(
                status.as_str(),
                "running" | "success" | "failure" | "cancelled"
            ) {
                let log = self.snapshot(run, job)?;
                let fresh = log.strip_prefix(&shown).unwrap_or(&log);
                print!("{fresh}");
                io::stdout().flush().context("unable to flush job log")?;
                shown = log;
            }
            match status.as_str() {
                "success" | "skipped" => return Ok(()),
                "failure" | "cancelled" => bail!("Forgejo action job ended with {status}"),
                "blocked" if done => bail!("Forgejo action job ended with blocked"),
                "unknown" | "waiting" | "running" | "blocked" => {}
                other => bail!("Forgejo action job has unknown status {other}"),
            }
            if Instant::now() >= deadline {
                bail!("Forgejo action job is still running past the watch timeout");
            }
            thread::sleep(poll);
        }
    }

    pub(super) fn jobs(&self, run: &str) -> Result<Value> {
        let attempt = self.flag("attempt").unwrap_or("1");
        let (owner, name) = super::pair(&self.repo()?)?;
        let origin = self.base.trim_end_matches("/api/v1");
        let value = self.send(
            "POST",
            &format!("{origin}/{owner}/{name}/actions/runs/{run}/jobs/0/attempt/{attempt}"),
            Some(&json!({"logCursors": []})),
        )?;
        let held = value
            .pointer("/state/run/jobs")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Forgejo action run response has no jobs"))?;
        let rows: Vec<Value> = held
            .iter()
            .enumerate()
            .map(|(index, job)| {
                json!({
                    "index": index,
                    "name": job.get("name").and_then(Value::as_str).unwrap_or(""),
                    "status": job.get("status").and_then(Value::as_str).unwrap_or(""),
                })
            })
            .collect();
        Ok(Value::Array(rows))
    }

    fn state(&self, run: &str, job: &str) -> Result<(String, bool)> {
        let attempt = self.flag("attempt").unwrap_or("1");
        let index = job.parse::<usize>().context("invalid job index")?;
        let (owner, name) = super::pair(&self.repo()?)?;
        let origin = self.base.trim_end_matches("/api/v1");
        let value = self.send(
            "POST",
            &format!("{origin}/{owner}/{name}/actions/runs/{run}/jobs/{job}/attempt/{attempt}"),
            Some(&json!({"logCursors": []})),
        )?;
        let status = value
            .pointer(&format!("/state/run/jobs/{index}/status"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Forgejo action job response has no status"))?;
        let done = value
            .pointer("/state/run/done")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok((status.to_string(), done))
    }

    fn snapshot(&self, run: &str, job: &str) -> Result<String> {
        match self.logged(run, job) {
            Ok(Value::String(log)) => Ok(log),
            Ok(other) => Ok(other.to_string()),
            Err(error) => Err(error),
        }
    }

    fn millis(&self, name: &str, fallback: u64) -> Result<Duration> {
        let value = self
            .flag(name)
            .map(str::parse)
            .transpose()
            .with_context(|| format!("invalid --{name}"))?
            .unwrap_or(fallback);
        Ok(Duration::from_millis(value))
    }
}
