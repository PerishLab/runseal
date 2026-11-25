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

#[test]
fn governance() {
    use crate::github::{Target, Window};
    let phase = r#"{"data":{"repository":{"issue":{"id":"I_9","number":9,"url":"https://github.com/PerishLab/concord/issues/9","repository":{"id":"R_1","nameWithOwner":"PerishLab/concord","owner":{"id":"O_1"}},"issueFieldValues":{"totalCount":0,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}}}"#;
    let badge =
        r#"{"data":{"repository":{"id":"R_1","label":{"id":"L_1","name":"acceptance:release"}}}}"#;
    let minted = r#"{"data":{"created":{"label":{"id":"L_2","name":"acceptance:source"}}}}"#;
    let relabeled = r#"{"data":{"alter":{"labelable":{"__typename":"Issue","id":"I_9"}}}}"#;
    let admission = r#"{"data":{"repository":{"id":"R_1","nameWithOwner":"PerishLab/concord","defaultBranchRef":{"name":"main"},"issue":{"id":"I_9","number":9,"url":"https://github.com/PerishLab/concord/issues/9","state":"OPEN","issueType":{"name":"Feature"},"subIssuesSummary":{"total":0},"labels":{"nodes":[],"pageInfo":{"hasNextPage":false}}},"rulesets":{"nodes":[],"pageInfo":{"hasNextPage":false}}}}}"#;
    let manifest =
        r#"{"data":{"repository":{"id":"R_1","object":{"oid":"abc","text":"[release]\n"}}}}"#;
    let closing = r#"{"data":{"repository":{"id":"R_1","pull":{"id":"PR_7","number":7,"url":"https://github.com/PerishLab/concord/pull/7","head":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","body":"","state":"OPEN","updated":"2026-11-25T00:00:00Z","references":{"total":0,"nodes":[],"page":{"next":false,"cursor":null}}}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", phase),
        reply("200 OK", "", badge),
        reply("200 OK", "", minted),
        reply("200 OK", "", relabeled),
        reply("200 OK", "", admission),
        reply("200 OK", "", manifest),
        reply("200 OK", "", closing),
    ]);
    let issue = Target::new("PerishLab", "concord", 9).unwrap();
    let pull = Target::new("PerishLab", "concord", 7).unwrap();
    assert_eq!(
        server.client.phase(&issue, false).unwrap().value["id"],
        "I_9"
    );
    assert_eq!(
        server
            .client
            .badge("PerishLab", "concord", "acceptance:release")
            .unwrap()
            .value
            .badge
            .unwrap()
            .id,
        "L_1"
    );
    assert_eq!(
        server
            .client
            .mint("R_1", "acceptance:source", "5319e7")
            .unwrap()
            .value
            .id,
        "L_2"
    );
    assert_eq!(
        server.client.relabel("I_9", "L_2", false).unwrap().outcome,
        Outcome::Accepted
    );
    assert_eq!(server.client.admission(&issue).unwrap().value["id"], "R_1");
    assert_eq!(
        server
            .client
            .manifest("PerishLab", "concord")
            .unwrap()
            .value
            .oid
            .as_deref(),
        Some("abc")
    );
    let window = Window::new(100, None).unwrap();
    assert_eq!(
        server.client.closing(&pull, &window).unwrap().value.pull["id"],
        "PR_7"
    );
    server.thread.join().unwrap();
}
