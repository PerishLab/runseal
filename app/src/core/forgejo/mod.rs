use std::{collections::BTreeMap, fs, process::Command};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::wire;

mod issue;

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    Call::parse(argv, vars)?.act()
}

pub(crate) struct Call {
    json: bool,
    pub(crate) base: String,
    auth: Auth,
    pub(crate) repo: Option<String>,
    pub(crate) limit: Option<usize>,
    pub(crate) title: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) state: Option<String>,
    deed: Deed,
}

enum Auth {
    File(String),
    Token(String),
}

enum Deed {
    User,
    Show(String),
    List,
    Create,
    Edit(String),
    Notes(String),
    Comment(String),
}

impl Call {
    fn parse(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Self> {
        let held = Flags::parse(argv)?;
        let Some(base) = held
            .base
            .or_else(|| vars.get("FORGEJO_URL").cloned())
            .filter(|held| !held.is_empty())
        else {
            bail!("@forgejo requires FORGEJO_URL or --url");
        };
        let auth = auth(&held.file, vars)?;
        Ok(Self {
            json: held.json,
            base: host(&base),
            auth,
            repo: held.repo,
            limit: held.limit,
            title: held.title,
            body: held.body,
            state: held.state,
            deed: deed(&held.rest)?,
        })
    }

    fn act(&self) -> Result<()> {
        let token = self.token()?;
        match &self.deed {
            Deed::User => self.emit("user", wire::get(&format!("{}/user", self.base), &token)?),
            Deed::Show(id) => self.emit("issue", self.shown(id, &token)?),
            Deed::List => self.emit("issues", self.listed(&token)?),
            Deed::Create => self.emit("issue", self.created(&token)?),
            Deed::Edit(id) => self.emit("issue", self.edited(id, &token)?),
            Deed::Notes(id) => self.emit("comments", self.notes(id, &token)?),
            Deed::Comment(id) => self.emit("comment", self.posted(id, &token)?),
        }
    }

    fn token(&self) -> Result<String> {
        match &self.auth {
            Auth::Token(held) => Ok(held.clone()),
            Auth::File(path) => load(path),
        }
    }

    fn emit(&self, kind: &str, value: Value) -> Result<()> {
        let value = self.clip(value);
        if self.json {
            println!("{}", wire::envelope(kind, &value)?);
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

    pub(crate) fn target(&self, id: &str) -> Result<(String, String, String)> {
        if let Some((repo, index)) = id.rsplit_once('#') {
            let (owner, name) = pair(repo)?;
            return Ok((owner, name, index.to_string()));
        }
        let (owner, name) = pair(&self.repo()?)?;
        Ok((owner, name, id.to_string()))
    }

    pub(crate) fn repo(&self) -> Result<String> {
        match &self.repo {
            Some(held) => Ok(held.clone()),
            None => remote(),
        }
    }
}

struct Flags {
    json: bool,
    base: Option<String>,
    file: Option<String>,
    repo: Option<String>,
    limit: Option<usize>,
    title: Option<String>,
    body: Option<String>,
    state: Option<String>,
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
            title: None,
            body: None,
            state: None,
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
            "--title" => self.title = Some(need(seen, "--title")?),
            "--body" => self.body = Some(need(seen, "--body")?),
            "--state" => self.state = Some(need(seen, "--state")?),
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

fn auth(file: &Option<String>, vars: &BTreeMap<String, String>) -> Result<Auth> {
    if let Some(file) = file.as_deref().filter(|held| !held.is_empty()) {
        return Ok(Auth::File(file.to_string()));
    }
    if let Some(token) = vars.get("FORGEJO_TOKEN").filter(|held| !held.is_empty()) {
        return Ok(Auth::Token(token.clone()));
    }
    if let Some(file) = vars
        .get("FORGEJO_TOKEN_FILE")
        .filter(|held| !held.is_empty())
    {
        return Ok(Auth::File(file.clone()));
    }
    bail!("@forgejo requires FORGEJO_TOKEN_FILE, FORGEJO_TOKEN, or --token-file")
}

fn deed(rest: &[String]) -> Result<Deed> {
    match rest {
        [kind, verb] if kind == "user" && verb == "show" => Ok(Deed::User),
        [kind, verb] if kind == "issue" && verb == "list" => Ok(Deed::List),
        [kind, verb] if kind == "issue" && verb == "create" => Ok(Deed::Create),
        [kind, verb, id] if kind == "issue" && verb == "show" => Ok(Deed::Show(id.clone())),
        [kind, verb, id] if kind == "issue" && verb == "edit" => Ok(Deed::Edit(id.clone())),
        [kind, verb, deed, id] if kind == "issue" && verb == "comment" && deed == "list" => {
            Ok(Deed::Notes(id.clone()))
        }
        [kind, verb, deed, id] if kind == "issue" && verb == "comment" && deed == "create" => {
            Ok(Deed::Comment(id.clone()))
        }
        _ => bail!(
            "@forgejo expected user show, issue show <id>, issue list, issue create, issue edit <id>, issue comment list <id>, or issue comment create <id>"
        ),
    }
}

fn load(path: &str) -> Result<String> {
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

pub(crate) fn pair(repo: &str) -> Result<(String, String)> {
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
