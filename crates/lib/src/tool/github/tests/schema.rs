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
