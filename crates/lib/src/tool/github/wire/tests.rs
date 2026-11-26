use super::observe::{Observation, graphql, route};
use crate::github::{
    Cause, Creation, Log, Outcome, Target, Visibility, records,
    tests::{self, reply, serve},
};
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
        route("post", "/orgs/SecretOrg/repos"),
        "rest.post.orgs.repos"
    );
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
    let repo =
        r#"{"id":14,"full_name":"SecretOrg/PrivateRepo","private":true,"default_branch":"main"}"#;
    let created = r#"{"id":15,"node_id":"R_created","full_name":"SecretOrg/PrivateRepo","private":true,"visibility":"private","archived":false,"disabled":false,"default_branch":"main","owner":{"id":9,"login":"SecretOrg","node_id":"O_secret","type":"Organization"}}"#;
    let partial = r#"{"data":{"organization":null},"errors":[{"message":"github_pat_fixture"}]}"#;
    let server = serve(vec![
        reply("200 OK", "", repo),
        reply("201 Created", "", created),
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
    let spec = Creation::new("PrivateRepo", Visibility::Private, Some("Private seed")).unwrap();
    client.establish("SecretOrg", &spec).unwrap();
    let _ = client.project("SecretOrg", 6).unwrap_err();
    let _ = client.repository("SecretOrg", "PrivateRepo").unwrap_err();
    let _ = client
        .logs("SecretOrg", "PrivateRepo", Log::inline(9, 1024).unwrap())
        .unwrap_err();
    assert_eq!(server.thread.join().unwrap().len(), 5);

    let events = drain();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0].provider, "github");
    assert_eq!(events[0].operation, "rest.get.repository");
    assert_eq!(events[0].transport, "rest");
    assert_eq!(events[0].outcome, Outcome::Observed);
    assert_eq!(events[0].cause, None);
    assert_eq!(events[0].status, Some(200));
    assert_eq!(events[0].attempts, 1);

    assert_eq!(events[1].operation, "rest.post.orgs.repos");
    assert_eq!(events[1].transport, "rest");
    assert_eq!(events[1].outcome, Outcome::Accepted);
    assert_eq!(events[1].status, Some(201));
    assert_eq!(events[1].attempts, 1);

    assert_eq!(events[2].operation, "graphql.RunsealProject");
    assert_eq!(events[2].transport, "graphql");
    assert_eq!(events[2].outcome, Outcome::Unknown);
    assert_eq!(events[2].cause, Some(Cause::Graphql));

    assert_eq!(events[3].outcome, Outcome::Refused);
    assert_eq!(events[3].cause, Some(Cause::Limited));
    assert_eq!(events[3].status, Some(403));
    assert_eq!(events[3].wait, Some(12));

    assert_eq!(events[4].operation, "rest.redirect.actions.jobs.logs");
    assert_eq!(events[4].transport, "redirect");
    assert_eq!(events[4].outcome, Outcome::Observed);
    assert_eq!(events[4].status, Some(302));

    let serialized = serde_json::to_string(&events).unwrap();
    for forbidden in [
        "SecretOrg",
        "PrivateRepo",
        "Private seed",
        "github_pat_fixture",
        "secret-body",
    ] {
        assert!(!serialized.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn transfer() {
    let headers = "Location: https://api.github.com/repos/PerishLab/folio/issues/1\r\nX-GitHub-Request-Id: 1234:ABCD\r\n";
    let server = serve(vec![reply("301 Moved Permanently", headers, "opaque body")]);
    let original = Target::new("PerishLab", ".github", 196).unwrap();
    let receipt = server.client.locate(&original).unwrap();
    assert_eq!(receipt.outcome, Outcome::Observed);
    assert_eq!(receipt.evidence.status, Some(301));
    assert_eq!(receipt.evidence.attempts, 1);
    assert_eq!(receipt.evidence.request.as_deref(), Some("1234:ABCD"));
    let records::Observation::Moved { moved } = &receipt.value else {
        panic!("expected transfer observation");
    };
    assert_eq!(moved.original, original);
    assert_eq!(moved.destination.owner(), "PerishLab");
    assert_eq!(moved.destination.repository(), "folio");
    assert_eq!(moved.destination.number(), 1);
    assert_eq!(
        serde_json::to_value(receipt).unwrap()["value"],
        json!({"moved": {
            "original": {"owner":"PerishLab","name":".github","number":196},
            "destination": {"owner":"PerishLab","name":"folio","number":1}
        }})
    );
    let requests = server.thread.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /repos/PerishLab/.github/issues/196 "));
}

#[test]
fn locations() {
    let locations = [
        "",
        "/repos/PerishLab/probe/issues/10",
        "http://api.github.com/repos/PerishLab/probe/issues/10",
        "https://foreign.test/repos/PerishLab/probe/issues/10",
        "https://api.github.com@foreign.test/repos/PerishLab/probe/issues/10",
        "https://api.github.com:443/repos/PerishLab/probe/issues/10",
        "https://api.github.com/repos/PerishLab/probe/pulls/10",
        "https://api.github.com/repositories/123/issues/10",
        "https://api.github.com/repos/PerishLab/%2e%2e/issues/10",
        "https://api.github.com/repos/PerishLab/probe/issues/0",
        "https://api.github.com/repos/PerishLab/probe/issues/010",
        "https://api.github.com/repos/PerishLab/probe/issues/2147483648",
        "https://api.github.com/repos/PerishLab/probe/issues/10/extra",
        "https://api.github.com/repos/PerishLab/probe/issues/10?raw=secret",
        "https://api.github.com/repos/PerishLab/probe/issues/10#secret",
        "https://api.github.com/repos/perishlab/PROBE/issues/9",
    ];
    let mut replies = vec![reply("301 Moved Permanently", "", "")];
    replies.extend(locations.map(|location| {
        reply(
            "301 Moved Permanently",
            &format!("Location: {location}\r\n"),
            "",
        )
    }));
    let server = serve(replies);
    let target = Target::new("PerishLab", "probe", 9).unwrap();
    for _ in 0..=locations.len() {
        let error = server.client.locate(&target).unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
        assert_eq!(error.outcome, Outcome::Unknown);
        assert_eq!(error.evidence.status, Some(301));
        assert_eq!(error.evidence.attempts, 1);
        assert!(!serde_json::to_string(&error).unwrap().contains("secret"));
    }
    let requests = server.thread.join().unwrap();
    assert_eq!(requests.len(), locations.len() + 1);
    assert!(
        requests
            .iter()
            .all(|call| call.starts_with("GET /repos/PerishLab/probe/issues/9 "))
    );
}

#[test]
fn observation() {
    let headers = "Location: https://api.github.com/repos/PerishLab/folio/issues/1\r\n";
    let mut foreign = tests::record(9);
    foreign["repository_url"] = json!("https://api.github.com/repos/Other/probe");
    let server = serve(vec![
        reply("200 OK", "", &tests::record(9).to_string()),
        reply("200 OK", headers, &foreign.to_string()),
        reply("404 Not Found", headers, ""),
        reply("403 Forbidden", headers, ""),
        reply("302 Found", headers, ""),
        reply("500 Internal Server Error", headers, ""),
        reply("301 Moved Permanently", headers, ""),
    ]);
    let target = Target::new("PerishLab", "probe", 9).unwrap();
    let receipt = server.client.locate(&target).unwrap();
    assert!(matches!(&receipt.value, records::Observation::Issue(record) if record.number == 9));
    let encoded = serde_json::to_value(receipt).unwrap();
    assert_eq!(encoded["value"]["node_id"], tests::record(9)["node_id"]);
    assert!(encoded["value"].get("moved").is_none());
    for (status, outcome, cause) in [
        (200, Outcome::Unknown, Cause::Shape),
        (404, Outcome::Refused, Cause::Http),
        (403, Outcome::Refused, Cause::Http),
        (302, Outcome::Unknown, Cause::Http),
        (500, Outcome::Unknown, Cause::Http),
    ] {
        let error = server.client.locate(&target).unwrap_err();
        assert_eq!(
            (error.evidence.status, error.outcome, error.cause),
            (Some(status), outcome, cause)
        );
    }
    let legacy = server.client.ticket(&target).unwrap_err();
    assert_eq!(
        (legacy.evidence.status, legacy.cause),
        (Some(301), Cause::Http)
    );
    assert_eq!(server.thread.join().unwrap().len(), 7);
}
