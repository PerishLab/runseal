use super::Call;
use crate::github::{Cause, Error, Outcome, Receipt};
use serde::Serialize;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Instant;

pub type Sink = fn(&Observation);

static SINK: OnceLock<Sink> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Observation {
    pub provider: &'static str,
    pub operation: String,
    pub transport: &'static str,
    pub outcome: Outcome,
    pub cause: Option<Cause>,
    pub status: Option<u16>,
    pub attempts: u8,
    #[serde(rename = "duration_ms")]
    pub elapsed: u64,
    pub partial: bool,
    pub wait: Option<u64>,
}

pub fn observe(sink: Sink) -> bool {
    SINK.set(sink).is_ok()
}

pub(super) fn capture<T>(
    sink: Option<Sink>,
    started: Instant,
    kind: (&'static str, String),
    result: &Result<Receipt<T>, Error>,
) {
    let Some(sink) = sink.or_else(|| SINK.get().copied()) else {
        return;
    };
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let (outcome, cause, evidence) = match result {
        Ok(receipt) => (receipt.outcome, None, &receipt.evidence),
        Err(error) => (error.outcome, Some(error.cause), &error.evidence),
    };
    let observation = Observation {
        provider: "github",
        operation: kind.1,
        transport: kind.0,
        outcome,
        cause,
        status: evidence.status,
        attempts: evidence.attempts,
        elapsed,
        partial: evidence.partial,
        wait: evidence.wait,
    };
    sink(&observation);
}

pub(super) fn operation(call: &Call<'_>) -> (&'static str, String) {
    match call {
        Call::Rest(path) | Call::Issue(path) | Call::Source(path) | Call::Recent(path, None) => {
            ("rest", route("get", path))
        }
        Call::Dispatch(path, _) | Call::Post(path, _) | Call::Recent(path, Some(_)) => {
            ("rest", route("post", path))
        }
        Call::Patch(path, _) => ("rest", route("patch", path)),
        Call::Put(path, _) => ("rest", route("put", path)),
        Call::Query(body) | Call::Raw(body) | Call::Survey(body) => {
            ("graphql", graphql(body, "query"))
        }
        Call::Mutation(body) => ("graphql", graphql(body, "mutation")),
    }
}

pub(super) fn route(method: &'static str, path: &str) -> String {
    let clean = path.split('?').next().unwrap_or(path);
    let parts: Vec<&str> = clean.split('/').filter(|part| !part.is_empty()).collect();
    if parts.as_slice() == ["user"] {
        return format!("rest.{method}.user");
    }
    if parts.first() == Some(&"orgs") && parts.len() == 3 && parts[2] == "repos" {
        return format!("rest.{method}.orgs.repos");
    }
    if parts.first() == Some(&"repos") && parts.len() >= 3 {
        if parts.len() == 3 {
            return format!("rest.{method}.repository");
        }
        let mut labels = Vec::new();
        for segment in &parts[3..] {
            if known(segment) {
                labels.push(*segment);
            }
        }
        if labels.is_empty() {
            return format!("rest.{method}.repository");
        }
        return format!("rest.{method}.{}", labels.join("."));
    }
    format!("rest.{method}.other")
}

pub(super) fn graphql(body: &Value, fallback: &str) -> String {
    let Some(query) = body.get("query").and_then(Value::as_str) else {
        return format!("graphql.{fallback}");
    };
    let trimmed = query.trim_start();
    let rest = trimmed
        .strip_prefix("query")
        .or_else(|| trimmed.strip_prefix("mutation"))
        .map(str::trim_start);
    let Some(rest) = rest else {
        return format!("graphql.{fallback}");
    };
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() {
        format!("graphql.{fallback}")
    } else {
        format!("graphql.{name}")
    }
}

fn known(segment: &str) -> bool {
    matches!(
        segment,
        "actions"
            | "artifacts"
            | "assets"
            | "attempts"
            | "blobs"
            | "branches"
            | "check-runs"
            | "check-suites"
            | "collaborators"
            | "comments"
            | "commits"
            | "compare"
            | "contents"
            | "dispatches"
            | "fields"
            | "git"
            | "heads"
            | "issues"
            | "jobs"
            | "logs"
            | "matching-refs"
            | "merge"
            | "permission"
            | "protection"
            | "pulls"
            | "ref"
            | "refs"
            | "releases"
            | "runs"
            | "status"
            | "statuses"
            | "tags"
            | "trees"
            | "values"
            | "workflows"
            | "zip"
    )
}
