use super::{sequence, words};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn pages() {
    let (url, handle) = sequence(vec![
        (
            "200 OK".into(),
            json!({"total_count":2,"workflow_runs":[{"id":1,"status":"success"}]}).to_string(),
        ),
        (
            "200 OK".into(),
            json!({"total_count":2,"workflow_runs":[{"id":2,"status":"running"}]}).to_string(),
        ),
    ]);
    let vars = BTreeMap::from([
        ("FORGEJO_URL".into(), url),
        ("FORGEJO_TOKEN".into(), "test".into()),
    ]);
    let reply = runseal::tool::call(
        "forgejo",
        &words(&["--repo", "PerishLab/plumb", "run", "list"]),
        &vars,
    )
    .unwrap();
    let seen = handle.join().unwrap();
    assert_eq!(reply.kind, "runs");
    assert_eq!(reply.value.as_array().unwrap().len(), 2);
    assert_eq!(reply.value[1]["status"], "running");
    assert!(seen[0].starts_with("GET /api/v1/repos/PerishLab/plumb/actions/runs?page=1&limit=50 "));
    assert!(seen[1].contains("page=2&limit=50"));
}

#[test]
fn capped() {
    let (url, handle) = sequence(vec![(
        "200 OK".into(),
        json!({"total_count":2,"workflow_runs":[{"id":1},{"id":2}]}).to_string(),
    )]);
    let vars = BTreeMap::from([
        ("FORGEJO_URL".into(), url),
        ("FORGEJO_TOKEN".into(), "test".into()),
    ]);
    let reply = runseal::tool::call(
        "forgejo",
        &words(&["--repo", "PerishLab/plumb", "run", "list", "--limit", "1"]),
        &vars,
    )
    .unwrap();
    handle.join().unwrap();
    assert_eq!(reply.value, json!([{"id":1}]));
}

#[test]
fn refuses() {
    for body in [
        json!({}),
        json!({"workflow_runs":[]}),
        json!({"total_count":1,"workflow_runs":[]}),
    ] {
        let (url, handle) = sequence(vec![("200 OK".into(), body.to_string())]);
        let vars = BTreeMap::from([
            ("FORGEJO_URL".into(), url),
            ("FORGEJO_TOKEN".into(), "test".into()),
        ]);
        assert!(
            runseal::tool::call(
                "forgejo",
                &words(&["--repo", "PerishLab/plumb", "run", "list"]),
                &vars
            )
            .is_err()
        );
        handle.join().unwrap();
    }
}
