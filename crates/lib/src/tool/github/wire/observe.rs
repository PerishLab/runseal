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
        Call::Rest(path) | Call::Source(path) | Call::Recent(path, None) => {
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

fn graphql(body: &Value, fallback: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::{Observation, graphql, route};
    use crate::github::{Cause, Log, Outcome, tests::reply, tests::serve};
    use serde_json::json;
    use std::cell::RefCell;

    thread_local! {
        static EVENTS: RefCell<Vec<Observation>> = const { RefCell::new(Vec::new()) };
    }

    fn record(event: &Observation) {
        EVENTS.with(|slot| slot.borrow_mut().push(event.clone()));
    }

    fn drain() -> Vec<Observation> {
        EVENTS.with(|slot| std::mem::take(&mut *slot.borrow_mut()))
    }

    #[test]
    fn sanitize() {
        assert_eq!(route("get", "/user"), "rest.get.user");
        assert_eq!(
            route("get", "/repos/SecretOrg/PrivateRepo"),
            "rest.get.repository"
        );
        assert_eq!(
            route(
                "redirect",
                "/repos/SecretOrg/PrivateRepo/actions/jobs/139001321286/logs"
            ),
            "rest.redirect.actions.jobs.logs"
        );
        assert_eq!(
            route(
                "post",
                "/repos/SecretOrg/PrivateRepo/actions/workflows/secret.yml/dispatches"
            ),
            "rest.post.actions.workflows.dispatches"
        );
        assert_eq!(
            route(
                "get",
                "/repos/SecretOrg/PrivateRepo/issues/42/comments?per_page=30&page=1"
            ),
            "rest.get.issues.comments"
        );
        assert_eq!(
            route(
                "get",
                "/repos/SecretOrg/PrivateRepo/contents/private/secret.txt?ref=deadbeef"
            ),
            "rest.get.contents"
        );
        assert_eq!(
            graphql(
                &json!({"query": "query RunsealProject($o: String!) { }"}),
                "query"
            ),
            "graphql.RunsealProject"
        );
        assert_eq!(
            graphql(
                &json!({"query": "mutation RunsealAmend($i: ID!) { }"}),
                "mutation"
            ),
            "graphql.RunsealAmend"
        );
        assert_eq!(
            graphql(&json!({"query": "{ viewer { login } }"}), "query"),
            "graphql.query"
        );
    }

    #[test]
    fn emission() {
        drain();
        let repo = r#"{"id":14,"full_name":"SecretOrg/PrivateRepo","private":true,"default_branch":"main"}"#;
        let partial =
            r#"{"data":{"organization":null},"errors":[{"message":"github_pat_fixture"}]}"#;
        let server = serve(vec![
            reply("200 OK", "", repo),
            reply("200 OK", "", partial),
            reply(
                "403 Forbidden",
                "Retry-After: 12\r\nX-RateLimit-Remaining: 0\r\n",
                "secret-body",
            ),
            reply("302 Found", "Location: https://evil.example/logs\r\n", ""),
        ]);
        let client = server.client.observer(record);
        client.repository("SecretOrg", "PrivateRepo").unwrap();
        let _ = client.project("SecretOrg", 6).unwrap_err();
        let _ = client.repository("SecretOrg", "PrivateRepo").unwrap_err();
        let _ = client
            .logs("SecretOrg", "PrivateRepo", Log::inline(9, 1024).unwrap())
            .unwrap_err();
        assert_eq!(server.thread.join().unwrap().len(), 4);

        let events = drain();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].provider, "github");
        assert_eq!(events[0].operation, "rest.get.repository");
        assert_eq!(events[0].transport, "rest");
        assert_eq!(events[0].outcome, Outcome::Observed);
        assert_eq!(events[0].cause, None);
        assert_eq!(events[0].status, Some(200));
        assert_eq!(events[0].attempts, 1);

        assert_eq!(events[1].operation, "graphql.RunsealProject");
        assert_eq!(events[1].transport, "graphql");
        assert_eq!(events[1].outcome, Outcome::Unknown);
        assert_eq!(events[1].cause, Some(Cause::Graphql));

        assert_eq!(events[2].outcome, Outcome::Refused);
        assert_eq!(events[2].cause, Some(Cause::Limited));
        assert_eq!(events[2].status, Some(403));
        assert_eq!(events[2].wait, Some(12));

        assert_eq!(events[3].operation, "rest.redirect.actions.jobs.logs");
        assert_eq!(events[3].transport, "redirect");
        assert_eq!(events[3].outcome, Outcome::Observed);
        assert_eq!(events[3].status, Some(302));

        let serialized = serde_json::to_string(&events).unwrap();
        for forbidden in [
            "SecretOrg",
            "PrivateRepo",
            "github_pat_fixture",
            "secret-body",
        ] {
            assert!(!serialized.contains(forbidden), "{forbidden}");
        }
    }
}
