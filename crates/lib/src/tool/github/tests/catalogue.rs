use super::{reply, serve};
use crate::github::{Cause, Outcome, Window, columns::Locator, view};

#[test]
fn columns() {
    let body = r#"[{"id":17,"node_id":"PVTF_example","name":"Priority"}]"#;
    let route = "https://api.github.com/organizations/17/projectsV2/6/fields";
    let valid = format!("Link: <{route}?per_page=1&after=cursor%2Bnext>; rel=\"next\"\r\n");
    let foreign = valid.replace("api.github.com", "example.invalid");
    let server = serve(vec![
        reply("200 OK", &valid, body),
        reply("200 OK", "", body),
        reply("200 OK", &foreign, body),
    ]);
    let project = Locator::new("PerishLab", 6, 17).unwrap();
    let first = server
        .client
        .columns(&project, &Window::new(1, None).unwrap())
        .unwrap();
    assert_eq!(first.value.next.as_deref(), Some("cursor+next"));
    let last = server
        .client
        .columns(
            &project,
            &Window::new(1, first.value.next.as_deref()).unwrap(),
        )
        .unwrap();
    assert!(last.value.next.is_none());
    let error = server
        .client
        .columns(&project, &Window::new(1, None).unwrap())
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let requests = server.thread.join().unwrap();
    assert!(requests[1].contains("per_page=1&after=cursor%2Bnext"));
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("x-github-api-version: 2026-03-10")
    );
}

#[test]
fn create() {
    let body = r#"{"value":{"id":17,"number":2,"node_id":"PVTV_example","name":"Sprint","layout":"table","project_url":"https://api.github.com/orgs/PerishLab/projectsV2/6"}}"#;
    let foreign = body.replace("projectsV2/6", "projectsV2/5");
    let server = serve(vec![
        reply("201 Created", "", body),
        reply("200 OK", "", body),
        reply("201 Created", "", &foreign),
        None,
    ]);
    let input = serde_json::from_str::<view::Create>(
        r#"{"name":"Sprint","layout":"table","visible_fields":[123],"sort_by":[[123,"asc"]]}"#,
    )
    .unwrap();
    let project = Locator::new("PerishLab", 6, 17).unwrap();
    let receipt = input.submit(&server.client, &project).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    for reason in [Cause::Shape, Cause::Shape, Cause::Network] {
        let failure = input.submit(&server.client, &project).unwrap_err();
        assert_eq!((failure.outcome, failure.cause), (Outcome::Unknown, reason));
        assert_eq!(failure.evidence.attempts, 1);
    }
    let calls = server.thread.join().unwrap();
    assert_eq!(calls.len(), 4);
    assert!(calls[0].starts_with("POST /orgs/PerishLab/projectsV2/6/views "));
    assert!(calls[0].contains("\"visible_fields\":[123]"));
}

#[test]
fn change() {
    let body = r#"{"data":{"change":{"projectV2View":{"id":"PVTV_example"}}}}"#;
    let server = serve(vec![reply("200 OK", "", body), reply("200 OK", "", body)]);
    let config = serde_json::from_str::<view::Change>(
        r#"{"operation":"update","view":"PVTV_example","layout":"board","visible":["PVTF_title"]}"#,
    )
    .unwrap();
    assert_eq!(
        config.submit(&server.client).unwrap().outcome,
        Outcome::Accepted
    );
    let remove = view::Change::Remove {
        view: "PVTV_example".into(),
    };
    assert_eq!(
        remove.submit(&server.client).unwrap().outcome,
        Outcome::Accepted
    );
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("\"layout\":\"BOARD_LAYOUT\""));
    assert!(calls[0].contains("\"visibleFieldIds\":[\"PVTF_title\"]"));
    assert!(calls[1].contains("deleteProjectV2View"));
}

#[test]
fn activity() {
    let removed = r#"{"data":{"change":{"id":"PVTW_demo","project":{"id":"PVT_target"}}}}"#;
    let published = r#"{"data":{"change":{"statusUpdate":{"id":"PVTSU_demo","body":"Progress","created":"2026-11-23","updated":"2026-11-23","project":{"id":"PVT_target"}}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", removed),
        reply("200 OK", "", published),
    ]);
    assert_eq!(
        server
            .client
            .retire("PVT_target", "PVTW_demo")
            .unwrap()
            .outcome,
        Outcome::Accepted
    );
    assert_eq!(
        server
            .client
            .record("PVT_target", "Progress")
            .unwrap()
            .outcome,
        Outcome::Accepted
    );
    let calls = server.thread.join().unwrap();
    assert!(calls[0].contains("deleteProjectV2Workflow"));
    assert!(calls[1].contains("createProjectV2StatusUpdate"));
}
