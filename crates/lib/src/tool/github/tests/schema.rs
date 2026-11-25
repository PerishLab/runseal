use super::{reply, serve};
use crate::github::{Cause, Outcome, Schema};

const FIELD: &str = r#"{"data":{"change":{"field":{"id":"PVTF_fixture","name":"Priority","issue":false,"project":{"id":"PVT_fixture"}}}}}"#;
const ATTACHED: &str = r#"{"data":{"change":{"field":{"id":"PVTF_issue","name":"Phase","issue":true,"origin":{"id":"IF_phase"},"project":{"id":"PVT_fixture"}}}}}"#;

fn request(input: &str) -> Schema {
    serde_json::from_str(input).unwrap()
}

#[test]
fn options() {
    let foreign = FIELD.replace("PVTF_fixture", "PVTF_foreign");
    let server = serve(vec![
        reply("200 OK", "", FIELD),
        reply("200 OK", "", &foreign),
        None,
    ]);
    let choice = r#"{"operation":"update","field":"PVTF_fixture","definition":{"type":"single","value":[{"id":"held","name":"High","color":"RED","description":""}]}}"#;
    let command = request(choice);
    let updated = server.client.schema(&command).unwrap();
    assert_eq!(updated.outcome, Outcome::Accepted);
    for cause in [Cause::Shape, Cause::Network] {
        let error = server.client.schema(&command).unwrap_err();
        assert_eq!((error.outcome, error.cause), (Outcome::Unknown, cause));
        assert_eq!(error.evidence.attempts, 1);
    }
    let requests = server.thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].contains("updateProjectV2Field"));
    assert!(requests[0].contains("\"singleSelectOptions\":[{"));
    assert!(requests[0].contains("\"id\":\"held\""));
}

#[test]
fn calendar() {
    let server = serve(vec![reply("200 OK", "", FIELD)]);
    let command = request(
        r#"{"operation":"create","project":"PVT_fixture","name":"Priority","definition":{"type":"iteration","value":{"startDate":"2026-11-23","duration":14,"iterations":[{"id":"original","title":"C1","duration":14,"startDate":"2026-11-23"}]}}}"#,
    );
    let receipt = server.client.schema(&command).unwrap();
    assert_eq!(receipt.value.id, "PVTF_fixture");
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("\"iterationConfiguration\""));
    assert!(requests[0].contains("\"id\":\"original\""));
    assert!(requests[0].contains("\"dataType\":\"ITERATION\""));
}

#[test]
fn issue() {
    let deleted = r#"{"data":{"change":{"field":{"id":"PVTF_issue"}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", ATTACHED),
        reply("200 OK", "", deleted),
    ]);
    let attach = request(r#"{"operation":"attach","project":"PVT_fixture","origin":"IF_phase"}"#);
    let attached = server.client.schema(&attach).unwrap();
    assert_eq!(attached.value.issue, Some(true));
    let remove = request(r#"{"operation":"remove","field":"PVTF_issue"}"#);
    let removed = server.client.schema(&remove).unwrap();
    assert_eq!(removed.outcome, Outcome::Accepted);
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("\"issueFieldId\":\"IF_phase\""));
    assert!(calls[1].contains("deleteProjectV2Field"));
}

#[test]
fn loopback() {
    use crate::github::Client;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    let remote = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 443);
    assert_eq!(
        Client::loopback("tok", remote).err().unwrap().cause,
        Cause::Input
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local = listener.local_addr().unwrap();
    let client = Client::loopback("tok", local).unwrap().timeout(5).unwrap();
    assert_eq!(client.timeout(0).err().unwrap().cause, Cause::Input);
    let client = Client::loopback("tok", local).unwrap();
    assert_eq!(client.timeout(301).err().unwrap().cause, Cause::Input);
}

#[test]
fn search() {
    use crate::github::Window;
    let body = r#"{"data":{"search":{"total":2,"page":{"more":true,"cursor":"c1"},"rows":[{"id":"I_1","number":12,"url":"https://github.com/PerishLab/concord/issues/12","title":"Title","body":"Body","state":"OPEN","updated":"2026-11-25T00:00:00Z","kind":{"name":"Feature"},"repository":{"name":"PerishLab/concord"}}]}}}"#;
    let server = serve(vec![reply("200 OK", "", body)]);
    let window = Window::new(10, None).unwrap();
    let found = server
        .client
        .search("repo:PerishLab/concord", &window)
        .unwrap();
    assert_eq!(found.outcome, Outcome::Observed);
    assert_eq!(found.value.total, 2);
    assert_eq!(found.value.next.as_deref(), Some("c1"));
    assert_eq!(found.value.rows[0].number, 12);
    server.thread.join().unwrap();
}

#[test]
fn classify() {
    use crate::github::classify::Status;
    let body = r#"{"data":{"repository":{"i1":{"__typename":"Issue","state":"OPEN","issueType":{"name":"Task"}},"i2":{"__typename":"PullRequest"},"i3":null}},"errors":[{"type":"NOT_FOUND","path":["repository","i3"],"message":"missing"}]}"#;
    let refused = r#"{"data":null,"errors":[{"type":"RATE_LIMITED","message":"slow"}]}"#;
    let silent = r#"{"data":{"repository":{"i1":null}}}"#;
    let server = serve(vec![
        reply("200 OK", "", body),
        reply("200 OK", "", refused),
        reply("200 OK", "", silent),
    ]);
    let observed = server
        .client
        .classify("PerishLab", "concord", &[1, 2, 3])
        .unwrap()
        .value;
    assert_eq!(
        observed[&1],
        Status::Issue {
            open: true,
            kind: Some("Task".into())
        }
    );
    assert_eq!(observed[&2], Status::Absent);
    assert_eq!(observed[&3], Status::Absent);
    assert_eq!(
        server
            .client
            .classify("PerishLab", "concord", &[1])
            .unwrap_err()
            .cause,
        Cause::Graphql
    );
    assert_eq!(
        server
            .client
            .classify("PerishLab", "concord", &[1])
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    server.thread.join().unwrap();
}

#[test]
fn attached() {
    use crate::github::{Number, Target};
    let body = r#"[{"issue_field_id":42,"data_type":"single_select","value":"Deliver"}]"#;
    let server = serve(vec![reply("200 OK", "", body)]);
    let target = Target::new("PerishLab", "concord", 7).unwrap();
    let found = server
        .client
        .attached(&target, Number::new(1, 100).unwrap())
        .unwrap()
        .value;
    assert_eq!(found.rows.len(), 1);
    assert_eq!(found.rows[0].field, 42);
    assert!(!found.more);
    server.thread.join().unwrap();
}

#[test]
fn annotation() {
    let sha = "a".repeat(40);
    let commit = "b".repeat(40);
    let body = format!(
        r#"{{"sha":"{sha}","tag":"v1.0.0","url":"https://api.github.com/repos/PerishLab/plumb/git/tags/{sha}","message":"v1.0.0\npromotes v1.0.0-rc.1\n","object":{{"sha":"{commit}","type":"commit","url":"https://api.github.com/repos/PerishLab/plumb/git/commits/{commit}"}}}}"#
    );
    let server = serve(vec![reply("200 OK", "", &body)]);
    let found = server
        .client
        .annotation("PerishLab", "plumb", &sha)
        .unwrap()
        .value;
    assert_eq!(
        found.message.as_deref(),
        Some("v1.0.0\npromotes v1.0.0-rc.1\n")
    );
    assert_eq!(
        found.object.url.as_deref(),
        Some(format!("https://api.github.com/repos/PerishLab/plumb/git/commits/{commit}").as_str())
    );
    server.thread.join().unwrap();
}
