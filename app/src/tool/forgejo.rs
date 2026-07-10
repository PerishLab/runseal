use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::Value;

use super::client::Client;

pub(super) fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    let value = match command {
        "repo" => repo(args)?,
        "pr" => pr(args)?,
        "secret" => secret(args)?,
        "variable" => variable(args)?,
        "workflow" => workflow(args)?,
        "run" => run(args)?,
        _ => bail!("unknown tool command: forgejo {command}"),
    };
    Ok(Some(serde_json::to_string(&value)?))
}

fn repo(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo repo get|create ...");
    };
    match command.as_str() {
        "get" => {
            let repo = target(rest)?;
            Client::load(rest)?.get(&repo.path(""), &[])
        }
        "create" => create(rest),
        _ => bail!("usage: runseal @tool forgejo repo get|create ..."),
    }
}

fn create(args: &[String]) -> Result<Value> {
    let owner = required(args, "--owner")?;
    let name = required(args, "--name")?;
    if owner != "@me" {
        atom(&owner)?;
    }
    atom(&name)?;
    let mut body = serde_json::json!({
        "name": name,
        "private": boolean(args, "--private")?.unwrap_or(false),
        "auto_init": false,
    });
    if let Some(description) = option(args, "--description") {
        body["description"] = description.into();
    }
    let path = if owner == "@me" {
        "/user/repos".to_string()
    } else {
        format!("/orgs/{owner}/repos")
    };
    Client::load(args)?.send(Method::POST, &path, &[], Some(body))
}

fn pr(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo pr find|create|get|merge|guard ...");
    };
    match command.as_str() {
        "find" => find(rest),
        "create" => open(rest),
        "get" => pull(rest),
        "merge" => merge(rest),
        "guard" => guard(rest),
        _ => bail!("usage: runseal @tool forgejo pr find|create|get|merge|guard ..."),
    }
}

fn find(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let head = required(args, "--head")?;
    let base = required(args, "--base")?;
    let query = vec![
        ("state".into(), "open".into()),
        ("limit".into(), "50".into()),
    ];
    let pulls = Client::load(args)?.get(&repo.path("pulls"), &query)?;
    let found = pulls.as_array().and_then(|pulls| {
        pulls.iter().find(|pull| {
            pull.pointer("/head/ref").and_then(Value::as_str) == Some(head.as_str())
                && pull.pointer("/base/ref").and_then(Value::as_str) == Some(base.as_str())
        })
    });
    Ok(found.cloned().unwrap_or(Value::Null))
}

fn open(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let mut body = serde_json::json!({
        "head": required(args, "--head")?,
        "base": required(args, "--base")?,
        "title": required(args, "--title")?,
    });
    if let Some(value) = option(args, "--body") {
        body["body"] = value.into();
    }
    Client::load(args)?.send(Method::POST, &repo.path("pulls"), &[], Some(body))
}

fn pull(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let number = number(args, "--number")?;
    Client::load(args)?.get(&repo.path(&format!("pulls/{number}")), &[])
}

fn merge(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let number = number(args, "--number")?;
    let body = serde_json::json!({
        "Do": "squash",
        "delete_branch_after_merge": boolean(args, "--delete-branch")?.unwrap_or(true),
        "head_commit_id": required(args, "--head")?,
    });
    Client::load(args)?.send(
        Method::POST,
        &repo.path(&format!("pulls/{number}/merge")),
        &[],
        Some(body),
    )
}

fn guard(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let number = number(args, "--number")?;
    let workflow = option(args, "--workflow").unwrap_or_else(|| "guard.yml".into());
    let interval = seconds(args, "--interval", 5)?;
    let timeout = seconds(args, "--timeout", 1800)?;
    let client = Client::load(args)?;
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
    wait(&client, &repo, &query, sha, &workflow, interval, timeout)
}

fn wait(
    client: &Client,
    repo: &Repo,
    query: &[(String, String)],
    sha: &str,
    workflow: &str,
    interval: Duration,
    timeout: Duration,
) -> Result<Value> {
    let deadline = Instant::now() + timeout;
    loop {
        let value = client.get(&repo.path("actions/runs"), query)?;
        if let Some(run) = select(&value, sha, workflow)
            && let Some(run) = outcome(run, workflow)?
        {
            return Ok(run);
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for Forgejo workflow {workflow}");
        }
        thread::sleep(interval);
    }
}

fn select<'a>(value: &'a Value, sha: &str, workflow: &str) -> Option<&'a Value> {
    value.get("workflow_runs")?.as_array()?.iter().find(|run| {
        run.get("commit_sha").and_then(Value::as_str) == Some(sha)
            && run.get("workflow_id").and_then(Value::as_str) == Some(workflow)
            && run.get("trigger_event").and_then(Value::as_str) == Some("pull_request")
    })
}

fn secret(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo secret upsert ...");
    };
    if command != "upsert" {
        bail!("usage: runseal @tool forgejo secret upsert ...");
    }
    let repo = target(rest)?;
    let name = required(rest, "--name")?;
    atom(&name)?;
    let body = serde_json::json!({ "data": content(rest)? });
    Client::load(rest)?.send(
        Method::PUT,
        &repo.path(&format!("actions/secrets/{name}")),
        &[],
        Some(body),
    )
}

fn variable(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo variable upsert ...");
    };
    if command != "upsert" {
        bail!("usage: runseal @tool forgejo variable upsert ...");
    }
    let repo = target(rest)?;
    let name = required(rest, "--name")?;
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
        Some(serde_json::json!({ "value": content(rest)? })),
    )
}

fn workflow(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo workflow dispatch ...");
    };
    if command != "dispatch" {
        bail!("usage: runseal @tool forgejo workflow dispatch ...");
    }
    let repo = target(rest)?;
    let workflow = required(rest, "--workflow")?;
    atom(&workflow)?;
    let body = serde_json::json!({
        "ref": required(rest, "--ref")?,
        "inputs": inputs(rest)?,
        "return_run_info": true,
    });
    Client::load(rest)?.send(
        Method::POST,
        &repo.path(&format!("actions/workflows/{workflow}/dispatches")),
        &[],
        Some(body),
    )
}

fn run(args: &[String]) -> Result<Value> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool forgejo run list|get|watch|cancel ...");
    };
    match command.as_str() {
        "list" => runs(rest),
        "get" => runget(rest),
        "watch" => watch(rest),
        "cancel" => bail!("Forgejo v15 has no supported workflow cancel API"),
        _ => bail!("usage: runseal @tool forgejo run list|get|watch|cancel ..."),
    }
}

fn runs(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let limit = option(args, "--limit")
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
        if let Some(value) = option(args, flag) {
            query.push((key.into(), value));
        }
    }
    query.push(("limit".into(), limit.to_string()));
    let mut value = Client::load(args)?.get(&repo.path("actions/runs"), &query)?;
    if let Some(runs) = value.get_mut("workflow_runs").and_then(Value::as_array_mut) {
        runs.truncate(limit);
    }
    Ok(value)
}

fn runget(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let id = number(args, "--id")?;
    Client::load(args)?.get(&repo.path(&format!("actions/runs/{id}")), &[])
}

fn watch(args: &[String]) -> Result<Value> {
    let repo = target(args)?;
    let id = number(args, "--id")?;
    let interval = seconds(args, "--interval", 10)?;
    let timeout = seconds(args, "--timeout", 3600)?;
    let client = Client::load(args)?;
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

fn target(args: &[String]) -> Result<Repo> {
    Repo::parse(&required(args, "--repo")?)
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

fn content(args: &[String]) -> Result<String> {
    let value = option(args, "--value");
    let env = option(args, "--value-env");
    let file = option(args, "--value-file");
    match (value, env, file) {
        (Some(value), None, None) => Ok(value),
        (None, Some(name), None) => {
            std::env::var(&name).with_context(|| format!("environment variable not set: {name}"))
        }
        (None, None, Some(path)) => std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read value file: {path}")),
        _ => bail!("pass exactly one of --value, --value-env, or --value-file"),
    }
}

fn inputs(args: &[String]) -> Result<serde_json::Map<String, Value>> {
    let mut inputs = serde_json::Map::new();
    for value in values(args, "--input") {
        let Some((key, value)) = value.split_once('=') else {
            bail!("--input expects key=value");
        };
        inputs.insert(key.into(), value.into());
    }
    Ok(inputs)
}

fn values(args: &[String], name: &str) -> Vec<String> {
    let prefix = format!("{name}=");
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            values.extend(args.get(index + 1).cloned());
            index += 2;
            continue;
        }
        if let Some(value) = args[index].strip_prefix(&prefix) {
            values.push(value.into());
        }
        index += 1;
    }
    values
}

fn required(args: &[String], name: &str) -> Result<String> {
    option(args, name).with_context(|| format!("{name} is required"))
}

fn option(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for (index, arg) in args.iter().enumerate() {
        if arg == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&prefix) {
            return Some(value.into());
        }
    }
    None
}

fn boolean(args: &[String], name: &str) -> Result<Option<bool>> {
    let Some(value) = option(args, name) else {
        return Ok(None);
    };
    match value.as_str() {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => bail!("{name} expects true or false"),
    }
}

fn number(args: &[String], name: &str) -> Result<u64> {
    required(args, name)?
        .parse()
        .with_context(|| format!("{name} expects an integer"))
}

fn seconds(args: &[String], name: &str, default: u64) -> Result<Duration> {
    let value = option(args, name)
        .map(|value| value.parse::<u64>())
        .transpose()
        .with_context(|| format!("{name} expects seconds"))?
        .unwrap_or(default);
    Ok(Duration::from_secs(value))
}
