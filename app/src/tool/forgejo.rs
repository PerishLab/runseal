use super::client::Client;
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::Value;
use std::{
    thread,
    time::{Duration, Instant},
};
pub(super) fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    let value = match command {
        "repo" => cmd(args).repo()?,
        "pr" => cmd(args).pr()?,
        "secret" => cmd(args).secret()?,
        "variable" => cmd(args).variable()?,
        "workflow" => cmd(args).workflow()?,
        "run" => cmd(args).run()?,
        _ => bail!("unknown tool command: forgejo {command}"),
    };
    Ok(Some(serde_json::to_string(&value)?))
}
struct Poll<'a> {
    client: &'a Client,
    repo: &'a Repo,
    interval: Duration,
    timeout: Duration,
}

impl Poll<'_> {
    fn wait(&self, query: &[(String, String)], sha: &str, workflow: &str) -> Result<Value> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let value = self.client.get(&self.repo.path("actions/runs"), query)?;
            if let Some(run) = select(&value, sha, workflow)
                && let Some(run) = outcome(run, workflow)?
            {
                return Ok(run);
            }
            if Instant::now() >= deadline {
                bail!("timed out waiting for Forgejo workflow {workflow}");
            }
            thread::sleep(self.interval);
        }
    }
}
fn select<'a>(value: &'a Value, sha: &str, workflow: &str) -> Option<&'a Value> {
    value.get("workflow_runs")?.as_array()?.iter().find(|run| {
        run.get("commit_sha").and_then(Value::as_str) == Some(sha)
            && run.get("workflow_id").and_then(Value::as_str) == Some(workflow)
            && run.get("trigger_event").and_then(Value::as_str) == Some("pull_request")
    })
}
fn outcome(run: &Value, label: &str) -> Result<Option<Value>> {
    match run.get("status").and_then(Value::as_str).unwrap_or("") {
        "success" => Ok(Some(run.clone())),
        "failure" | "cancelled" | "skipped" => {
            bail!("Forgejo workflow {label} failed: {run}")
        }
        _ => Ok(None),
    }
}
struct Repo {
    owner: String,
    name: String,
}
impl Repo {
    fn parse(value: &str) -> Result<Self> {
        let mut parts = value.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if owner.is_empty() || name.is_empty() || parts.next().is_some() {
            bail!("--repo expects owner/name");
        }
        atom(owner)?;
        atom(name)?;
        Ok(Self {
            owner: owner.into(),
            name: name.into(),
        })
    }

    fn path(&self, suffix: &str) -> String {
        let base = format!("/repos/{}/{}", self.owner, self.name);
        if suffix.is_empty() {
            base
        } else {
            format!("{base}/{suffix}")
        }
    }
}
fn atom(value: &str) -> Result<()> {
    if value
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
    {
        return Ok(());
    }
    bail!("invalid Forgejo path atom: {value}")
}

struct Cmd<'a> {
    args: &'a [String],
}

fn cmd(args: &[String]) -> Cmd<'_> {
    Cmd { args }
}

impl Cmd<'_> {
    fn repo(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo repo get|create ...");
        };
        match command.as_str() {
            "get" => {
                let repo = cmd(rest).target()?;
                Client::load(rest)?.get(&repo.path(""), &[])
            }
            "create" => cmd(rest).create(),
            _ => bail!("usage: runseal @tool forgejo repo get|create ..."),
        }
    }

    fn create(&self) -> Result<Value> {
        let owner = self.required("--owner")?;
        let name = self.required("--name")?;
        if owner != "@me" {
            atom(&owner)?;
        }
        atom(&name)?;
        let mut body = serde_json::json!({
            "name": name,
            "private": self.boolean("--private")?.unwrap_or(false),
            "auto_init": false,
        });
        if let Some(description) = self.option("--description") {
            body["description"] = description.into();
        }
        let path = if owner == "@me" {
            "/user/repos".to_string()
        } else {
            format!("/orgs/{owner}/repos")
        };
        Client::load(self.args)?.send(Method::POST, &path, &[], Some(body))
    }

    fn pr(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo pr find|create|get|merge|guard ...");
        };
        match command.as_str() {
            "find" => cmd(rest).find(),
            "create" => cmd(rest).open(),
            "get" => cmd(rest).pull(),
            "merge" => cmd(rest).merge(),
            "guard" => cmd(rest).guard(),
            _ => bail!("usage: runseal @tool forgejo pr find|create|get|merge|guard ..."),
        }
    }

    fn find(&self) -> Result<Value> {
        let repo = self.target()?;
        let head = self.required("--head")?;
        let base = self.required("--base")?;
        let query = vec![
            ("state".into(), "open".into()),
            ("limit".into(), "50".into()),
        ];
        let pulls = Client::load(self.args)?.get(&repo.path("pulls"), &query)?;
        let found = pulls.as_array().and_then(|pulls| {
            pulls.iter().find(|pull| {
                pull.pointer("/head/ref").and_then(Value::as_str) == Some(head.as_str())
                    && pull.pointer("/base/ref").and_then(Value::as_str) == Some(base.as_str())
            })
        });
        Ok(found.cloned().unwrap_or(Value::Null))
    }

    fn open(&self) -> Result<Value> {
        let repo = self.target()?;
        let mut body = serde_json::json!({
            "head": self.required("--head")?,
            "base": self.required("--base")?,
            "title": self.required("--title")?,
        });
        if let Some(value) = self.option("--body") {
            body["body"] = value.into();
        }
        Client::load(self.args)?.send(Method::POST, &repo.path("pulls"), &[], Some(body))
    }

    fn pull(&self) -> Result<Value> {
        let repo = self.target()?;
        let number = self.number("--number")?;
        Client::load(self.args)?.get(&repo.path(&format!("pulls/{number}")), &[])
    }

    fn merge(&self) -> Result<Value> {
        let repo = self.target()?;
        let number = self.number("--number")?;
        let body = serde_json::json!({
            "Do": "squash",
            "delete_branch_after_merge": self.boolean("--delete-branch")?.unwrap_or(true),
            "head_commit_id": self.required("--head")?,
        });
        Client::load(self.args)?.send(
            Method::POST,
            &repo.path(&format!("pulls/{number}/merge")),
            &[],
            Some(body),
        )
    }

    fn guard(&self) -> Result<Value> {
        let repo = self.target()?;
        let number = self.number("--number")?;
        let workflow = self
            .option("--workflow")
            .unwrap_or_else(|| "guard.yml".into());
        let interval = self.seconds("--interval", 5)?;
        let timeout = self.seconds("--timeout", 1800)?;
        let client = Client::load(self.args)?;
        let pull = client.get(&repo.path(&format!("pulls/{number}")), &[])?;
        let sha = pull
            .pointer("/head/sha")
            .and_then(Value::as_str)
            .context("Forgejo pull request payload missing head.sha")?;
        let query = vec![
            ("head_sha".into(), sha.into()),
            ("workflow_id".into(), workflow.clone()),
            ("limit".into(), "10".into()),
        ];
        Poll {
            client: &client,
            repo: &repo,
            interval,
            timeout,
        }
        .wait(&query, sha, &workflow)
    }

    fn secret(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo secret upsert ...");
        };
        if command != "upsert" {
            bail!("usage: runseal @tool forgejo secret upsert ...");
        }
        let repo = cmd(rest).target()?;
        let name = cmd(rest).required("--name")?;
        atom(&name)?;
        let body = serde_json::json!({ "data": cmd(rest).content()? });
        Client::load(rest)?.send(
            Method::PUT,
            &repo.path(&format!("actions/secrets/{name}")),
            &[],
            Some(body),
        )
    }

    fn variable(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo variable upsert ...");
        };
        if command != "upsert" {
            bail!("usage: runseal @tool forgejo variable upsert ...");
        }
        let repo = cmd(rest).target()?;
        let name = cmd(rest).required("--name")?;
        atom(&name)?;
        let path = repo.path(&format!("actions/variables/{name}"));
        let client = Client::load(rest)?;
        let method = if client.probe(&path)?.is_some() {
            Method::PUT
        } else {
            Method::POST
        };
        client.send(
            method,
            &path,
            &[],
            Some(serde_json::json!({ "value": cmd(rest).content()? })),
        )
    }

    fn workflow(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo workflow dispatch ...");
        };
        if command != "dispatch" {
            bail!("usage: runseal @tool forgejo workflow dispatch ...");
        }
        let repo = cmd(rest).target()?;
        let workflow = cmd(rest).required("--workflow")?;
        atom(&workflow)?;
        let body = serde_json::json!({
            "ref": cmd(rest).required("--ref")?,
            "inputs": cmd(rest).inputs()?,
            "return_run_info": true,
        });
        Client::load(rest)?.send(
            Method::POST,
            &repo.path(&format!("actions/workflows/{workflow}/dispatches")),
            &[],
            Some(body),
        )
    }

    fn run(&self) -> Result<Value> {
        let [command, rest @ ..] = self.args else {
            bail!("usage: runseal @tool forgejo run list|get|watch|cancel ...");
        };
        match command.as_str() {
            "list" => cmd(rest).runs(),
            "get" => cmd(rest).get(),
            "watch" => cmd(rest).watch(),
            "cancel" => bail!("Forgejo v15 has no supported workflow cancel API"),
            _ => bail!("usage: runseal @tool forgejo run list|get|watch|cancel ..."),
        }
    }

    fn runs(&self) -> Result<Value> {
        let repo = self.target()?;
        let limit = self
            .option("--limit")
            .map(|value| value.parse::<usize>())
            .transpose()
            .context("--limit expects an integer")?
            .unwrap_or(20);
        if limit == 0 {
            bail!("--limit expects a positive integer");
        }
        let mut query = Vec::new();
        for (flag, key) in [
            ("--event", "event"),
            ("--status", "status"),
            ("--sha", "head_sha"),
            ("--ref", "ref"),
            ("--workflow", "workflow_id"),
            ("--number", "run_number"),
        ] {
            if let Some(value) = self.option(flag) {
                query.push((key.into(), value));
            }
        }
        query.push(("limit".into(), limit.to_string()));
        let mut value = Client::load(self.args)?.get(&repo.path("actions/runs"), &query)?;
        if let Some(runs) = value.get_mut("workflow_runs").and_then(Value::as_array_mut) {
            runs.truncate(limit);
        }
        Ok(value)
    }

    fn get(&self) -> Result<Value> {
        let repo = self.target()?;
        let id = self.number("--id")?;
        Client::load(self.args)?.get(&repo.path(&format!("actions/runs/{id}")), &[])
    }

    fn watch(&self) -> Result<Value> {
        let repo = self.target()?;
        let id = self.number("--id")?;
        let interval = self.seconds("--interval", 10)?;
        let timeout = self.seconds("--timeout", 3600)?;
        let client = Client::load(self.args)?;
        let deadline = Instant::now() + timeout;
        loop {
            let run = client.get(&repo.path(&format!("actions/runs/{id}")), &[])?;
            if let Some(run) = outcome(&run, "run")? {
                return Ok(run);
            }
            if Instant::now() >= deadline {
                bail!("timed out waiting for Forgejo workflow run {id}");
            }
            thread::sleep(interval);
        }
    }

    fn target(&self) -> Result<Repo> {
        Repo::parse(&self.required("--repo")?)
    }

    fn content(&self) -> Result<String> {
        let value = self.option("--value");
        let env = self.option("--value-env");
        let file = self.option("--value-file");
        match (value, env, file) {
            (Some(value), None, None) => Ok(value),
            (None, Some(name), None) => std::env::var(&name)
                .with_context(|| format!("environment variable not set: {name}")),
            (None, None, Some(path)) => std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read value file: {path}")),
            _ => bail!("pass exactly one of --value, --value-env, or --value-file"),
        }
    }

    fn inputs(&self) -> Result<serde_json::Map<String, Value>> {
        let mut inputs = serde_json::Map::new();
        for value in self.values("--input") {
            let Some((key, value)) = value.split_once('=') else {
                bail!("--input expects key=value");
            };
            inputs.insert(key.into(), value.into());
        }
        Ok(inputs)
    }

    fn values(&self, name: &str) -> Vec<String> {
        let prefix = format!("{name}=");
        let mut values = Vec::new();
        let mut index = 0;
        while index < self.args.len() {
            if self.args[index] == name {
                values.extend(self.args.get(index + 1).cloned());
                index += 2;
                continue;
            }
            if let Some(value) = self.args[index].strip_prefix(&prefix) {
                values.push(value.into());
            }
            index += 1;
        }
        values
    }

    fn required(&self, name: &str) -> Result<String> {
        self.option(name)
            .with_context(|| format!("{name} is required"))
    }

    fn option(&self, name: &str) -> Option<String> {
        let prefix = format!("{name}=");
        for (index, arg) in self.args.iter().enumerate() {
            if arg == name {
                return self.args.get(index + 1).cloned();
            }
            if let Some(value) = arg.strip_prefix(&prefix) {
                return Some(value.into());
            }
        }
        None
    }

    fn boolean(&self, name: &str) -> Result<Option<bool>> {
        let Some(value) = self.option(name) else {
            return Ok(None);
        };
        match value.as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => bail!("{name} expects true or false"),
        }
    }

    fn number(&self, name: &str) -> Result<u64> {
        self.required(name)?
            .parse()
            .with_context(|| format!("{name} expects an integer"))
    }

    fn seconds(&self, name: &str, default: u64) -> Result<Duration> {
        let value = self
            .option(name)
            .map(|value| value.parse::<u64>())
            .transpose()
            .with_context(|| format!("{name} expects seconds"))?
            .unwrap_or(default);
        Ok(Duration::from_secs(value))
    }
}
