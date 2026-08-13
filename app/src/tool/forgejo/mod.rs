use std::{collections::BTreeMap, fs, process::Command};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::Reply;
use crate::{http, parse};

mod deed;
mod flow;
mod help;
mod issue;
mod job;
mod pull;
mod repo;

use deed::{Deed, Kind, deed, kind};

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    Seat::open(argv, vars)?.act()
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    if argv.iter().any(|arg| arg == "--help") {
        print!("{}", help::TEXT);
        return Ok(());
    }
    let seat = Seat::open(argv, vars)?;
    let deed = deed(&seat.line.rest)?;
    if seat.line.watch {
        return match deed {
            Deed::Job(run, job) => seat.follow(&run, &job),
            _ => bail!("@forgejo --watch requires job log RUN JOB"),
        };
    }
    let reply = seat.act()?;
    reply.emit(argv)
}

struct Seat {
    line: parse::Line,
    base: String,
    auth: Option<String>,
}

impl Seat {
    fn open(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Self> {
        let line = parse::Line::take(argv)?;
        if matches!(line.rest.first().map(String::as_str), Some("get")) {
            return Ok(Self {
                line,
                base: String::new(),
                auth: None,
            });
        }
        let Some(url) = line
            .url
            .clone()
            .or_else(|| vars.get("FORGEJO_URL").cloned())
            .filter(|held| !held.is_empty())
        else {
            bail!("@forgejo requires FORGEJO_URL or --url");
        };
        Ok(Self {
            auth: Some(token(&line, vars)?),
            base: format!("{}/api/v1", url.trim_end_matches('/')),
            line,
        })
    }

    fn act(&self) -> Result<Reply> {
        if self.line.watch {
            bail!("structured @forgejo calls cannot watch a job log");
        }
        let deed = deed(&self.line.rest)?;
        let value = self.work(&deed)?;
        Ok(Reply {
            kind: kind(&deed),
            value: self.clip(value),
        })
    }

    fn work(&self, deed: &Deed) -> Result<Value> {
        match deed {
            Deed::User => self.send("GET", &format!("{}/user", self.base), None),
            Deed::Show(id) => self.shown(id),
            Deed::List => self.listed(),
            Deed::Create => self.created(),
            Deed::Edit(id) => self.edited(id),
            Deed::Notes(id) => self.notes(id),
            Deed::Comment(id) => self.posted(id),
            Deed::Pull(Kind::Show(id)) => self.opened(id),
            Deed::Pull(Kind::List) => self.pulls(),
            Deed::Pull(Kind::Create) => self.raised(),
            Deed::Pull(Kind::Edit(id)) => self.amended(id),
            Deed::Pull(Kind::Merge(id)) => self.merged(id),
            Deed::Status(id) => self.standing(id),
            Deed::Branch(Kind::Show(id)) => self.stem(id),
            Deed::Branch(Kind::Set(id)) => self.forked(id),
            Deed::Guard(Kind::Show(id)) => self.guard(id),
            Deed::Guard(Kind::Create) => self.shield(),
            Deed::Guard(Kind::Edit(id)) => self.ruled(id),
            Deed::Repo(Kind::Show(_)) => self.home(),
            Deed::Repo(Kind::Edit(_)) => self.patched(),
            Deed::Repo(Kind::Drop(_)) => self.removed(),
            Deed::Secret(Kind::List) => self.secrets(),
            Deed::Secret(Kind::Set(id)) => self.stored(id),
            Deed::Secret(Kind::Drop(id)) => self.cleared(id),
            Deed::Flow(id) => self.sent(id),
            Deed::Run(id) => self.running(id),
            Deed::Job(run, job) => self.logged(run, job),
            Deed::Task(run) => self.tasks(run),
            Deed::Review(Kind::Show(id)) => self.reviews(id),
            Deed::Review(Kind::Set(id)) => self.reviewed(id),
            Deed::Label => self.labels(),
            Deed::Fetch(url) => http::send("GET", url, None, None),
            _ => bail!("@forgejo expected a known resource verb"),
        }
    }

    fn clip(&self, value: Value) -> Value {
        let Some(limit) = self.line.limit else {
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

    fn send(&self, verb: &str, url: &str, body: Option<&Value>) -> Result<Value> {
        http::send(verb, url, self.auth.as_deref(), body)
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
        match &self.line.repo {
            Some(held) => Ok(held.clone()),
            None => remote(),
        }
    }

    fn pages(&self, stem: &str) -> Result<Value> {
        let mut items = Vec::new();
        let mut turn = 1usize;
        loop {
            let value = self.send("GET", &format!("{stem}&page={turn}"), None)?;
            let Some(rows) = value.as_array() else {
                bail!("forgejo list did not return an array");
            };
            let count = rows.len();
            items.extend(rows.iter().cloned());
            if count < 50 || self.line.limit.is_some_and(|limit| items.len() >= limit) {
                break;
            }
            turn += 1;
        }
        Ok(Value::Array(items))
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.line.flag(name)
    }
}

impl Reply {
    fn emit(self, argv: &[String]) -> Result<()> {
        let line = parse::Line::take(argv)?;
        if line.json {
            println!("{}", parse::dump(self.kind, &self.value)?);
        } else {
            parse::show(&self.value);
        }
        Ok(())
    }
}

fn token(line: &parse::Line, vars: &BTreeMap<String, String>) -> Result<String> {
    let secret = if let Some(file) = line.file.as_deref().filter(|held| !held.is_empty()) {
        load(file)?
    } else if let Some(held) = vars.get("FORGEJO_TOKEN").filter(|held| !held.is_empty()) {
        held.clone()
    } else if let Some(file) = vars
        .get("FORGEJO_TOKEN_FILE")
        .filter(|held| !held.is_empty())
    {
        load(file)?
    } else {
        bail!("@forgejo requires FORGEJO_TOKEN_FILE, FORGEJO_TOKEN, or --token-file")
    };
    Ok(format!("token {secret}"))
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

fn pair(repo: &str) -> Result<(String, String)> {
    let Some((owner, name)) = repo.split_once('/') else {
        bail!("repository must be owner/name: {repo}");
    };
    if owner.is_empty() || name.is_empty() {
        bail!("repository must be owner/name: {repo}");
    }
    Ok((owner.into(), name.into()))
}

fn quote(value: &str) -> String {
    value
        .bytes()
        .map(|held| match held {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (held as char).to_string()
            }
            _ => format!("%{held:02X}"),
        })
        .collect()
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
