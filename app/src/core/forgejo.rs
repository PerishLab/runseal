use std::{collections::BTreeMap, fs, process::Command};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::wire;

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    Call::parse(argv, vars)?.act()
}

struct Call {
    json: bool,
    base: String,
    file: String,
    repo: Option<String>,
    limit: Option<usize>,
    deed: Deed,
}

enum Deed {
    User,
    Show(String),
    List,
}

impl Call {
    fn parse(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Self> {
        let held = Flags::parse(argv)?;
        let base = held.base.or_else(|| vars.get("FORGEJO_URL").cloned());
        let file = held
            .file
            .or_else(|| vars.get("FORGEJO_TOKEN_FILE").cloned());
        let Some(base) = base.filter(|held| !held.is_empty()) else {
            bail!("@forgejo requires FORGEJO_URL or --url");
        };
        let Some(file) = file.filter(|held| !held.is_empty()) else {
            bail!("@forgejo requires FORGEJO_TOKEN_FILE or --token-file");
        };
        Ok(Self {
            json: held.json,
            base: host(&base),
            file,
            repo: held.repo,
            limit: held.limit,
            deed: deed(&held.rest)?,
        })
    }

    fn act(&self) -> Result<()> {
        let token = secret(&self.file)?;
        match &self.deed {
            Deed::User => self.emit(wire::get(&format!("{}/user", self.base), &token)?),
            Deed::Show(id) => self.emit(wire::get(&self.shown(id)?, &token)?),
            Deed::List => self.emit(self.listed(&token)?),
        }
    }

    fn shown(&self, id: &str) -> Result<String> {
        let (owner, name, index) = self.target(id)?;
        Ok(format!("{}/repos/{owner}/{name}/issues/{index}", self.base))
    }

    fn target(&self, id: &str) -> Result<(String, String, String)> {
        if let Some((repo, index)) = id.rsplit_once('#') {
            let (owner, name) = pair(repo)?;
            return Ok((owner, name, index.to_string()));
        }
        let (owner, name) = pair(&self.repo()?)?;
        Ok((owner, name, id.to_string()))
    }

    fn repo(&self) -> Result<String> {
        match &self.repo {
            Some(held) => Ok(held.clone()),
            None => remote(),
        }
    }

    fn listed(&self, token: &str) -> Result<Value> {
        let mut items = Vec::new();
        let mut turn = 1usize;
        loop {
            let rows = self.batch(token, turn)?;
            let count = rows.len();
            items.extend(rows);
            if !self.more(items.len(), count) {
                break;
            }
            turn += 1;
        }
        Ok(Value::Array(items))
    }

    fn batch(&self, token: &str, turn: usize) -> Result<Vec<Value>> {
        let (owner, name) = pair(&self.repo()?)?;
        let url = format!(
            "{}/repos/{owner}/{name}/issues?state=all&limit=50&page={turn}",
            self.base
        );
        let value = wire::get(&url, token)?;
        let Some(rows) = value.as_array() else {
            bail!("forgejo issue list did not return an array");
        };
        Ok(rows.clone())
    }

    fn more(&self, total: usize, count: usize) -> bool {
        count >= 50 && self.limit.is_none_or(|limit| total < limit)
    }

    fn emit(&self, value: Value) -> Result<()> {
        let value = self.clip(value);
        if self.json {
            println!("{}", wire::envelope(&value)?);
            return Ok(());
        }
        wire::print(&value);
        Ok(())
    }

    fn clip(&self, value: Value) -> Value {
        let Some(limit) = self.limit else {
            return value;
        };
        match value {
            Value::Array(mut rows) => {
                rows.truncate(limit);
                Value::Array(rows)
            }
            other => other,
        }
    }
}

struct Flags {
    json: bool,
    base: Option<String>,
    file: Option<String>,
    repo: Option<String>,
    limit: Option<usize>,
    rest: Vec<String>,
}

impl Flags {
    fn parse(argv: &[String]) -> Result<Self> {
        let mut held = Self {
            json: false,
            base: None,
            file: None,
            repo: None,
            limit: None,
            rest: Vec::new(),
        };
        let mut seen = argv.iter();
        while let Some(arg) = seen.next() {
            held.step(arg, &mut seen)?;
        }
        Ok(held)
    }

    fn step(&mut self, arg: &str, seen: &mut std::slice::Iter<String>) -> Result<()> {
        match arg {
            "--json" => self.json = true,
            "--url" => self.base = Some(need(seen, "--url")?),
            "--token-file" => self.file = Some(need(seen, "--token-file")?),
            "--repo" => self.repo = Some(need(seen, "--repo")?),
            "--limit" => {
                self.limit = Some(need(seen, "--limit")?.parse().context("invalid --limit")?)
            }
            _ => self.rest.push(arg.to_string()),
        }
        Ok(())
    }
}

fn need(seen: &mut std::slice::Iter<String>, flag: &str) -> Result<String> {
    seen.next()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn deed(rest: &[String]) -> Result<Deed> {
    match rest {
        [kind, verb] if kind == "user" && verb == "show" => Ok(Deed::User),
        [kind, verb] if kind == "issue" && verb == "list" => Ok(Deed::List),
        [kind, verb, id] if kind == "issue" && verb == "show" => Ok(Deed::Show(id.clone())),
        _ => bail!("@forgejo expected user show, issue show <id>, or issue list"),
    }
}

fn secret(path: &str) -> Result<String> {
    let held =
        fs::read_to_string(path).with_context(|| format!("unable to read token file {path}"))?;
    let token = held.trim();
    if token.is_empty() {
        bail!("token file is empty: {path}");
    }
    Ok(token.to_string())
}

fn host(url: &str) -> String {
    format!("{}/api/v1", url.trim_end_matches('/'))
}

fn pair(repo: &str) -> Result<(String, String)> {
    let Some((owner, name)) = repo.split_once('/') else {
        bail!("repository must be owner/name: {repo}");
    };
    if owner.is_empty() || name.is_empty() {
        bail!("repository must be owner/name: {repo}");
    }
    Ok((owner.into(), name.into()))
}

fn remote() -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("failed to read git remote origin")?;
    if !output.status.success() {
        bail!("git remote origin is unavailable; pass --repo owner/name");
    }
    peel(&String::from_utf8_lossy(&output.stdout))
}

fn peel(raw: &str) -> Result<String> {
    let held = raw.trim().trim_end_matches(".git");
    if let Some(path) = held.strip_prefix("git@") {
        let Some((_, path)) = path.split_once(':') else {
            bail!("unable to parse git remote: {raw}");
        };
        return pair(path).map(|(owner, name)| format!("{owner}/{name}"));
    }
    if let Some(rest) = held.split_once("://").map(|part| part.1) {
        let mut parts = rest.split('/').skip(1);
        let owner = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        return pair(&format!("{owner}/{name}")).map(|(owner, name)| format!("{owner}/{name}"));
    }
    bail!("unable to parse git remote: {raw}")
}
