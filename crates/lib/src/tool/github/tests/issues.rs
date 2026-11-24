use super::{Cause, Outcome, reply, serve};
use crate::github::{Relation, Target, Window, phase::Update};
use serde_json::{Value, json};

const FIRST: &str = "I_sample";
const OTHER: &str = "I_other";

fn target() -> Target {
    Target::new("perishlab", "runseal", 229).unwrap()
}

fn envelope(issue: Value) -> String {
    let repository = json!({"nameWithOwner":"PerishLab/runseal","issue":issue});
    json!({"data":{"repository":repository}}).to_string()
}

fn listing(rows: Value, next: Option<&str>) -> String {
    let page = json!({"more":next.is_some(),"cursor":next});
    envelope(json!({"id":FIRST,"number":229,"list":{"rows":rows,"total":2,"page":page}}))
}

fn comment(id: &str) -> Value {
    json!({"id":id,"body":"observed","url":"https://github.com/PerishLab/runseal/issues/229",
          "created":"2026-01-02T00:00:00Z","updated":"2026-01-02T00:00:00Z"})
}

#[test]
fn issue() {
    let value = envelope(json!({
        "id":FIRST,"number":229,"title":"fixture","body":"scope","url":"https://github.com/PerishLab/runseal/issues/229",
        "state":"OPEN","reason":null,"updated":"2026-01-02T00:00:00Z","kind":{"name":"Feature"},"repository":{"id":"R_fixture"}
    }));
    let server = serve(vec![reply("200 OK", "", &value)]);
    assert_eq!(server.client.issue(&target()).unwrap().value.id, FIRST);
    let calls = server.thread.join().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].contains("query RunsealIssue"));
}

#[test]
fn pagination() {
    let one = listing(json!([comment("IC_one")]), Some("Y3Vyc29y"));
    let two = listing(json!([comment("IC_two")]), None);
    let server = serve(vec![reply("200 OK", "", &one), reply("200 OK", "", &two)]);
    let window = Window::new(1, None).unwrap();
    let page = server.client.comments(&target(), &window).unwrap().value;
    assert_eq!(page.target, FIRST);
    assert_eq!(page.rows[0].id, "IC_one");
    let window = Window::new(1, page.next.as_deref()).unwrap();
    let page = server.client.comments(&target(), &window).unwrap().value;
    assert_eq!(page.rows[0].id, "IC_two");
    assert!(page.next.is_none());
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("list:comments(first:"));
    assert!(calls[1].contains(r#""after":"Y3Vyc29y""#));
}

#[test]
fn stalled() {
    let body = listing(json!([comment("IC_one")]), Some("cursor"));
    let server = serve(vec![reply("200 OK", "", &body)]);
    let window = Window::new(1, Some("cursor")).unwrap();
    let error = server.client.comments(&target(), &window).unwrap_err();
    assert_eq!(
        (error.outcome, error.cause),
        (Outcome::Unknown, Cause::Shape)
    );
    assert_eq!(server.thread.join().unwrap().len(), 1);
}

#[test]
fn refused() {
    assert!(Window::new(101, None).is_err());
    assert!(Window::new(1, Some("bad cursor")).is_err());
    let body = listing(
        json!([{"id":OTHER,"state":"OPEN","url":"https://github.com/a/b/issues/1"}]),
        None,
    );
    let server = serve(vec![reply("200 OK", "", &body)]);
    let wrong = Target::new("perishlab", "elsewhere", 229).unwrap();
    let window = Window::new(5, None).unwrap();
    let error = server
        .client
        .relations(&wrong, &window, Relation::Blocker)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains("list:blockedBy"));
}

#[test]
fn link() {
    let payload = json!({"issue":{"id":FIRST},"related":{"id":OTHER}});
    let good = json!({"data":{"link":payload}}).to_string();
    let server = serve(vec![reply("200 OK", "", &good)]);
    let receipt = server.client.link(FIRST, OTHER, Relation::Child).unwrap();
    assert_eq!(receipt.value.related.id, OTHER);
    assert!(server.thread.join().unwrap()[0].contains("replaceParent:false"));
}

#[test]
fn post() {
    let payload = json!({"subject":{"id":FIRST},"commentEdge":{"node":comment("IC_one")}});
    let good = json!({"data":{"addComment":payload}}).to_string();
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &good),
        None,
    ]);
    assert_eq!(
        server.client.comment(FIRST, "observed").unwrap().value.id,
        "IC_one"
    );
    let mismatch = server.client.comment(OTHER, "observed").unwrap_err();
    assert_eq!(mismatch.cause, Cause::Shape);
    let lost = server.client.comment(FIRST, "observed").unwrap_err();
    assert_eq!(lost.outcome, Outcome::Unknown);
    assert_eq!(lost.evidence.attempts, 1);
    assert_eq!(server.thread.join().unwrap().len(), 3);
}

#[test]
fn kinds() {
    let good = r#"{"data":{"organization":{"id":"O_lab","login":"PerishLab","list":{"total":2,"rows":[{"id":"IT_example","name":"Task","enabled":true}],"page":{"more":true,"cursor":"page-two"}}}}}"#;
    let foreign = good.replace("PerishLab", "Foreign");
    let server = serve(vec![
        reply("200 OK", "", good),
        reply("200 OK", "", &foreign),
    ]);
    let window = Window::new(1, None).unwrap();
    let receipt = server.client.kinds("perishlab", &window).unwrap();
    assert_eq!(receipt.value.target, "O_lab");
    assert_eq!(receipt.value.next.as_deref(), Some("page-two"));
    assert_eq!(receipt.value.rows[0].id, "IT_example");
    assert!(receipt.value.rows[0].enabled);
    let error = server.client.kinds("perishlab", &window).unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains("issueTypes(first:$size,after:$after)"));
}

#[test]
fn definitions() {
    let body = r#"{"data":{"organization":{"id":"O_lab","login":"PerishLab","list":{"total":2,"rows":[{"kind":"IssueFieldSingleSelect","id":"IF_phase","name":"Phase","datatype":"SINGLE_SELECT","visibility":"ORGANIZATION","options":[{"id":"option","name":"Deliver"}]}],"page":{"more":true,"cursor":"after"}}}}}"#;
    let foreign = body.replace("PerishLab", "Foreign");
    let server = serve(vec![
        reply("200 OK", "", body),
        reply("200 OK", "", &foreign),
    ]);
    let window = Window::new(1, None).unwrap();
    let got = server.client.definitions("PerishLab", &window).unwrap();
    assert_eq!(got.value.next.as_deref(), Some("after"));
    assert_eq!(got.value.rows[0].options.as_ref().unwrap()[0].id, "option");
    assert_eq!(
        server
            .client
            .definitions("PerishLab", &window)
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    assert!(server.thread.join().unwrap()[0].contains("issueFields(first:$size,after:$after)"));
}

#[test]
fn observations() {
    let body = r#"{"data":{"repository":{"nameWithOwner":"PerishLab/runseal","issue":{"id":"I_anchor","number":229,"list":{"total":1,"rows":[{"kind":"IssueFieldSingleSelectValue","id":"IFV_example","option":"option","name":"Deliver","field":{"id":"IF_phase"}}],"page":{"more":false,"cursor":null}}}}}}"#;
    let server = serve(vec![reply("200 OK", "", body)]);
    let target = Target::new("PerishLab", "runseal", 229).unwrap();
    let got = server
        .client
        .observations(&target, &Window::new(10, None).unwrap())
        .unwrap();
    assert_eq!(got.value.target, "I_anchor");
    assert_eq!(got.value.rows[0].option.as_deref(), Some("option"));
    assert!(
        server.thread.join().unwrap()[0].contains("issueFieldValues(first:$size,after:$after)")
    );
}

#[test]
fn selection() {
    let body = r#"{"data":{"change":{"issue":{"id":"I_anchor"}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", body),
        reply("200 OK", "", body),
        None,
    ]);
    let mut request = Update {
        issue: "I_anchor".into(),
        field: "IF_phase".into(),
        option: Some("option".into()),
        rationale: "Verified".into(),
    };
    assert_eq!(
        server.client.select(&request).unwrap().outcome,
        Outcome::Accepted
    );
    request.option = None;
    assert_eq!(
        server.client.select(&request).unwrap().outcome,
        Outcome::Accepted
    );
    let error = server.client.select(&request).unwrap_err();
    assert_eq!(
        (error.outcome, error.cause),
        (Outcome::Unknown, Cause::Network)
    );
    assert_eq!(error.evidence.attempts, 1);
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("\"singleSelectOptionId\":\"option\""));
    assert!(requests[1].contains("\"delete\":true"));
    assert_eq!(requests.len(), 3);
}

#[test]
fn explicit() {
    let missing = r#"{"issue":"I_anchor","field":"IF_phase","rationale":"Reviewed"}"#;
    let clear = r#"{"issue":"I_anchor","field":"IF_phase","option":null,"rationale":"Reviewed"}"#;
    assert!(serde_json::from_str::<Update>(missing).is_err());
    assert!(
        serde_json::from_str::<Update>(clear)
            .unwrap()
            .option
            .is_none()
    );
}

#[test]
fn freshness() {
    use crate::github::acceptance::Pair;

    let repo = r#"{"id":"R_anchor","nameWithOwner":"PerishLab/runseal","issue":{"id":"I_review","number":229,"title":"Delivery","body":"Body","url":"https://github.com/PerishLab/runseal/issues/229","state":"OPEN","updated":"2026-11-23T00:00:00Z","edited":null,"repository":{"id":"R_anchor"},"list":{"total":2,"rows":[{"id":"RE_example","created":"2026-11-23T00:00:00Z"}],"page":{"more":true,"cursor":"resume"}}}}"#;
    let comment = r#"{"id":"IC_declaration","body":"Conditions","url":"https://github.com/PerishLab/runseal/issues/229#issuecomment-4","created":"2026-11-23T00:00:00Z","updated":"2026-11-23T00:00:00Z","edited":null,"issue":{"id":"I_review","repository":{"id":"R_anchor"}}}"#;
    let closure = comment.replace("IC_declaration", "IC_closure");
    let good = format!(
        r#"{{"data":{{"repository":{repo},"declaration":{comment},"closure":{closure}}}}}"#
    );
    let wrong = good.replacen(&closure, &closure.replace("I_review", "I_other"), 1);
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &wrong),
    ]);
    let pair = Pair::new("IC_declaration", "IC_closure").unwrap();
    let window = Window::new(1, None).unwrap();
    let observed = server
        .client
        .freshness(&target(), &window, &pair)
        .unwrap()
        .value;
    assert_eq!(observed.history.events.next.as_deref(), Some("resume"));
    assert_eq!(observed.history.events.rows[0].id, "RE_example");
    assert_eq!(observed.declaration.id, "IC_declaration");
    assert!(observed.closure.edited.is_none());
    assert_eq!(
        server
            .client
            .freshness(&target(), &window, &pair)
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("itemTypes:[REOPENED_EVENT]"));
    assert!(calls[0].contains("edited:lastEditedAt"));
}

#[test]
fn review() {
    let good = r#"{"data":{"node":{"id":"IC_review","body":"Proof","url":"https://github.com/PerishLab/runseal/issues/229#issuecomment-5","created":"2026-11-23T00:00:00Z","updated":"2026-11-23T00:00:00Z","edited":null,"issue":{"id":"I_review","repository":{"id":"R_anchor"}}}}}"#;
    let missing = good.replace("\"edited\":null,", "");
    let server = serve(vec![
        reply("200 OK", "", good),
        reply("200 OK", "", &missing),
    ]);
    let comment = server.client.review("I_review", "IC_review").unwrap().value;
    assert_eq!(comment.issue.repository.id, "R_anchor");
    assert!(comment.edited.is_none());
    assert_eq!(
        server
            .client
            .review("I_review", "IC_review")
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    assert!(server.thread.join().unwrap()[0].contains("node(id:$comment)"));
}
