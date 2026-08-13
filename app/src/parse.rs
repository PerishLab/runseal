use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::{Value, json};

pub struct Line {
    pub json: bool,
    pub watch: bool,
    pub url: Option<String>,
    pub file: Option<String>,
    pub repo: Option<String>,
    pub limit: Option<usize>,
    pub flags: BTreeMap<String, String>,
    pub rest: Vec<String>,
}

impl Line {
    pub fn take(argv: &[String]) -> Result<Self> {
        let mut held = Self {
            json: false,
            watch: false,
            url: None,
            file: None,
            repo: None,
            limit: None,
            flags: BTreeMap::new(),
            rest: Vec::new(),
        };
        let mut seen = argv.iter();
        while let Some(arg) = seen.next() {
            held.step(arg, &mut seen)?;
        }
        Ok(held)
    }

    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    fn step(&mut self, arg: &str, seen: &mut std::slice::Iter<String>) -> Result<()> {
        match arg {
            "--json" => self.json = true,
            "--watch" => self.watch = true,
            "--url" => self.url = Some(need(seen, "--url")?),
            "--token-file" => self.file = Some(need(seen, "--token-file")?),
            "--repo" => self.repo = Some(need(seen, "--repo")?),
            "--limit" => {
                self.limit = Some(need(seen, "--limit")?.parse().context("invalid --limit")?)
            }
            flag if flag.starts_with("--") => {
                let name = flag.trim_start_matches('-');
                self.flags.insert(name.to_string(), need(seen, flag)?);
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

pub fn dump(kind: &str, value: &Value) -> Result<String> {
    serde_json::to_string(&json!({"version": 1, kind: value})).context("unable to encode JSON")
}

pub fn show(value: &Value) {
    match value {
        Value::Null => println!("ok"),
        Value::String(text) => print!("{text}"),
        Value::Array(rows) => {
            for row in rows {
                println!("{}", line(row));
            }
        }
        other => println!("{}", line(other)),
    }
}

fn line(value: &Value) -> String {
    if let Some(login) = value.get("login").and_then(Value::as_str) {
        return login.to_string();
    }
    if let Some(title) = value.get("title").and_then(Value::as_str) {
        let number = value
            .get("number")
            .map(|held| held.to_string())
            .unwrap_or_default();
        let state = value.get("state").and_then(Value::as_str).unwrap_or("");
        return format!("{number}\t{state}\t{title}");
    }
    if let Some(name) = value
        .get("name")
        .or_else(|| value.get("full_name"))
        .or_else(|| value.get("rule_name"))
        .or_else(|| value.get("branch_name"))
        .and_then(Value::as_str)
    {
        return name.to_string();
    }
    if let Some(state) = value.get("state").and_then(Value::as_str)
        && value.get("statuses").is_some()
    {
        return state.to_string();
    }
    let id = value
        .get("id")
        .or_else(|| value.get("number"))
        .map(|held| held.to_string())
        .unwrap_or_default();
    let login = value
        .pointer("/user/login")
        .and_then(Value::as_str)
        .unwrap_or("");
    let body = value
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("");
    if id.is_empty() && login.is_empty() && body.is_empty() {
        return value.to_string();
    }
    format!("{id}\t{login}\t{body}")
}
