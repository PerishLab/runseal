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
    if let Some(name) = value.get("name").and_then(Value::as_str) {
        return match value.get("status").and_then(Value::as_str) {
            Some(status) => format!("\t{status}\t{name}"),
            None => name.to_string(),
        };
    }
    match value.get("id") {
        Some(id) => format!("{id}\t\t"),
        None => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{dump, line};
    use serde_json::json;

    #[test]
    fn records() {
        for (value, expected) in [
            (
                json!({"id":"token-id","name":"observer","status":"active"}),
                "\tactive\tobserver",
            ),
            (json!({"id":"bucket-id","name":"assets"}), "assets"),
            (json!({"id":"token-id"}), "\"token-id\"\t\t"),
        ] {
            assert_eq!(line(&value), expected);
        }
    }

    #[test]
    fn opaque() {
        for value in [
            json!({"login":"fixture"}),
            json!({"number":1,"title":"fixture","state":"open"}),
            json!({"full_name":"fixture","branch_name":"main","rule_name":"fixture"}),
            json!({"state":"open","statuses":[],"user":{"login":"fixture"},"body":"fixture"}),
        ] {
            assert_eq!(line(&value), value.to_string());
            let output: serde_json::Value =
                serde_json::from_str(&dump("fixture", &value).unwrap()).unwrap();
            assert_eq!(output, json!({"version":1,"fixture":value}));
        }
        assert_eq!(
            line(&json!({"name":"observer","status":"active","run_number":9,"index":8})),
            "\tactive\tobserver"
        );
    }
}
