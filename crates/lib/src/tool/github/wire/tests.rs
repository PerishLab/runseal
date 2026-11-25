use super::observe::{Observation, graphql, route};
use crate::github::{
    Cause, Creation, Log, Outcome, Visibility,
    tests::{reply, serve},
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
