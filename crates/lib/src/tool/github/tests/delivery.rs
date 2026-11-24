use super::{reply, serve};
use crate::github::{Cause, Draft, Filter, Inventory, Outcome, Signal, Verdict};
use serde_json::{Value, json};

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn creation() {
    let created = r#"{"data":{"createIssue":{"issue":{"id":"I_new","number":42,"title":"Delivery","body":"Body","url":"https://github.com/PerishLab/runseal/issues/42","state":"OPEN","updated":"2026-11-23T00:00:00Z","repository":{"id":"R_runseal"},"kind":{"id":"IT_task","name":"Task"}}}}}"#;
    let wrong = created.replace("IT_task", "IT_other");
    let server = serve(vec![
        reply("200 OK", "", created),
        reply("200 OK", "", &wrong),
        None,
    ]);
    let draft = Draft::new("R_runseal", "Delivery", "Body", Some("IT_task")).unwrap();
    let receipt = server.client.create(&draft).unwrap();
    assert_eq!(receipt.value.number, 42);
    assert_eq!(receipt.value.kind.unwrap().name, "Task");
    for cause in [Cause::Shape, Cause::Network] {
        let error = server.client.create(&draft).unwrap_err();
        assert_eq!(error.cause, cause);
        assert_eq!(error.outcome, Outcome::Unknown);
        assert_eq!(error.evidence.attempts, 1);
    }
    let calls = server.thread.join().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("POST /graphql "));
    assert!(calls[0].contains("\"issueTypeId\":\"IT_task\""));
    assert!(calls[0].contains("\"repositoryId\":\"R_runseal\""));
}

fn status() -> Value {
    json!({
        "url":format!("https://api.github.com/repos/PerishLab/runseal/statuses/{HEAD}"),
        "id":51,"state":"success","context":"guard / guard (pull_request)",
        "description":"Proof admitted","target_url":"https://github.com/PerishLab/runseal/actions/runs/3",
    })
}

#[test]
fn marks() {
    let row = status();
    let changed = row.to_string().replace(HEAD, &"b".repeat(40));
    let server = serve(vec![
        reply("201 Created", "", &row.to_string()),
        reply("201 Created", "", &changed),
        None,
    ]);
    let signal = Signal::new(
        Verdict::Success,
        row["context"].as_str().unwrap(),
        row["description"].as_str(),
        row["target_url"].as_str(),
    )
    .unwrap();
    let receipt = server
        .client
        .mark("PerishLab", "runseal", HEAD, &signal)
        .unwrap();
    assert_eq!(receipt.value.id, 51);
    for cause in [Cause::Shape, Cause::Network] {
        let error = server
            .client
            .mark("PerishLab", "runseal", HEAD, &signal)
            .unwrap_err();
        assert_eq!(error.cause, cause);
        assert_eq!(error.outcome, Outcome::Unknown);
        assert_eq!(error.evidence.attempts, 1);
    }
    let calls = server.thread.join().unwrap();
    assert!(calls[0].starts_with(&format!("POST /repos/PerishLab/runseal/statuses/{HEAD} ")));
    assert!(calls[0].contains("\"state\":\"success\""));
}

#[test]
fn statuses() {
    let page =
        json!({"state":"success","sha":HEAD,"total_count":1,"statuses":[status()],"repository":{"id":19,"full_name":"PerishLab/runseal"}}).to_string();
    let changed = page.replace(HEAD, &"b".repeat(40));
    let server = serve(vec![
        reply("200 OK", "", &page),
        reply("200 OK", "", &changed),
    ]);
    let query = Filter::new(HEAD, 1, 20).unwrap();
    let receipt = server
        .client
        .statuses("PerishLab", "runseal", &query)
        .unwrap();
    assert_eq!(
        receipt.value.rows[0].context,
        "guard / guard (pull_request)"
    );
    let error = server
        .client
        .statuses("PerishLab", "runseal", &query)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains(&format!("/commits/{HEAD}/status?per_page=20&page=1")));
}

#[test]
fn artifacts() {
    let row = json!({
        "id":71,"name":"guard","size_in_bytes":800,"expired":false,
        "digest":format!("sha256:{}", "a".repeat(64)),
        "url":"https://api.github.com/repos/PerishLab/runseal/actions/artifacts/71",
        "workflow_run":{"id":17,"repository_id":19,"head_sha":HEAD},
    });
    let page = json!({"total_count":1,"artifacts":[row]}).to_string();
    let changed = page.replace("\"id\":17", "\"id\":18");
    let server = serve(vec![
        reply("200 OK", "", &page),
        reply("200 OK", "", &changed),
        reply("200 OK", "", &row.to_string()),
    ]);
    let query = Inventory::new(Some(17), 1, 20)
        .unwrap()
        .named("guard")
        .unwrap();
    let receipt = server
        .client
        .artifacts("PerishLab", "runseal", query.clone())
        .unwrap();
    assert_eq!(receipt.value.rows[0].id, 71);
    let error = server
        .client
        .artifacts("PerishLab", "runseal", query)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let receipt = server.client.artifact("PerishLab", "runseal", 71).unwrap();
    assert_eq!(receipt.value.run.unwrap().head, HEAD);
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("/actions/runs/17/artifacts?per_page=20&page=1&name=guard"));
    assert!(calls[2].contains("/actions/artifacts/71 HTTP/1.1"));
}

fn issue(number: u32) -> Value {
    json!({
        "node_id":format!("I_{number}"),"number":number,"title":"Follow",
        "html_url":format!("https://github.com/PerishLab/probe/issues/{number}"),
        "repository_url":"https://api.github.com/repos/PerishLab/probe",
        "body":null,"updated_at":"2026-11-24T00:00:00Z","state":"open",
        "state_reason":null,"type":{"name":"Auto"},"labels":[],
        "user":{"id":19,"login":"fixture[bot]"},
    })
}

#[test]
fn inventory() {
    let first = json!([{"number":8,"pull_request":{"url":"https://api.github.com/repos/PerishLab/probe/pulls/8"}}]);
    let second = json!([issue(9)]);
    let more = "Link: <https://api.github.com/repositories/52/issues?page=2>; rel=\"next\"\r\n";
    let server = serve(vec![
        reply("200 OK", more, &first.to_string()),
        reply("200 OK", "", &second.to_string()),
    ]);
    let first = server
        .client
        .issues("PerishLab", "probe", 1, 1)
        .unwrap()
        .value;
    assert!(first.rows.is_empty());
    let next = first.next.unwrap();
    assert_eq!(next, 2);
    let second = server
        .client
        .issues("PerishLab", "probe", next, 1)
        .unwrap()
        .value;
    assert!(second.next.is_none());
    assert_eq!(second.rows[0].kind.as_ref().unwrap().name, "Auto");
    assert!(second.rows[0].body.is_none());
    let requests = server.thread.join().unwrap();
    assert!(
        requests[1].starts_with("GET /repos/PerishLab/probe/issues?state=open&per_page=1&page=2 ")
    );
}

#[test]
fn record() {
    let mut unknown = issue(9);
    unknown.as_object_mut().unwrap().remove("type");
    let mut foreign = issue(9);
    foreign["repository_url"] = json!("https://api.github.com/repos/Other/probe");
    let mut pull = issue(9);
    pull["pull_request"] = json!({});
    let replies =
        [issue(9), foreign, unknown, pull].map(|value| reply("200 OK", "", &value.to_string()));
    let server = serve(replies.into());
    let target = crate::github::Target::new("PerishLab", "probe", 9).unwrap();
    let record = server.client.ticket(&target).unwrap().value;
    assert_eq!(record.user.unwrap().id, 19);
    for _ in 0..3 {
        assert_eq!(
            server.client.ticket(&target).unwrap_err().cause,
            Cause::Shape
        );
    }
    assert_eq!(server.thread.join().unwrap().len(), 4);
}

#[test]
fn viewer() {
    let actor = json!({"id":"U_bot","login":"fixture[bot]","databaseId":19});
    let data = json!({"viewer":actor});
    let exact = json!({"data":data});
    let partial = json!({"data":data,"errors":[{"message":"partial"}]});
    let absent = json!({"data":{"viewer":null}});
    let replies = [exact, partial, absent].map(|value| reply("200 OK", "", &value.to_string()));
    let server = serve(replies.into());
    let viewer = server.client.viewer().unwrap().value;
    assert_eq!(viewer.number, 19);
    for cause in [Cause::Graphql, Cause::Shape] {
        assert_eq!(server.client.viewer().unwrap_err().cause, cause);
    }
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("viewer { id databaseId login }"));
}
