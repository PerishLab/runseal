use super::{reply, serve};
use crate::github::{Cause, Confirmation, Outcome, Settings, Structure};

const PROJECT: &str = r#"{"id":"PVT_fixture","number":6,"title":"Delivery","url":"https://github.com/orgs/PerishLab/projects/6","updated":"2026-11-23T00:00:00Z","closed":false,"public":false,"writable":true,"description":null,"readme":"","owner":{"kind":"Organization","id":"O_fixture","login":"PerishLab"}}"#;

fn answer(project: &str) -> String {
    format!(r#"{{"data":{{"change":{{"project":{project}}}}}}}"#)
}

#[test]
fn create() {
    let good = answer(PROJECT);
    let foreign = answer(&PROJECT.replace("O_fixture", "O_foreign"));
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &foreign),
        None,
    ]);
    let command = Structure::Create {
        owner: "O_fixture".into(),
        title: "Delivery".into(),
    };
    let receipt = server.client.structure(&command).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    let Confirmation::Project { project } = receipt.value else {
        panic!("missing project");
    };
    assert_eq!(project.id, "PVT_fixture");
    for cause in [Cause::Shape, Cause::Network] {
        let error = server.client.structure(&command).unwrap_err();
        assert_eq!((error.outcome, error.cause), (Outcome::Unknown, cause));
        assert_eq!(error.evidence.attempts, 1);
    }
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("createProjectV2"));
    assert!(requests[0].contains("\"ownerId\":\"O_fixture\""));
}

#[test]
fn configure() {
    let good = answer(PROJECT);
    let server = serve(vec![reply("200 OK", "", &good)]);
    let command = Structure::Configure {
        project: "PVT_fixture".into(),
        settings: Settings {
            title: None,
            description: Some(String::new()),
            readme: None,
            public: Some(false),
            closed: None,
        },
    };
    let receipt = server.client.structure(&command).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("updateProjectV2"));
    assert!(requests[0].contains("\"public\":false"));
    assert!(requests[0].contains("\"shortDescription\":\"\""));
    assert!(!requests[0].contains("\"closed\":"));
}

#[test]
fn repositories() {
    let body = r#"{"data":{"change":{"repository":{"id":"R_fixture"}}}}"#;
    let server = serve(vec![reply("200 OK", "", body), reply("200 OK", "", body)]);
    for present in [true, false] {
        let command = Structure::Relate {
            project: "PVT_fixture".into(),
            repository: "R_fixture".into(),
            present,
        };
        let receipt = server.client.structure(&command).unwrap();
        assert_eq!(receipt.outcome, Outcome::Accepted);
        assert!(
            matches!(receipt.value, Confirmation::Repository {present:value, ..} if value==present)
        );
    }
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("change:linkProjectV2ToRepository"));
    assert!(requests[1].contains("change:unlinkProjectV2FromRepository"));
}

#[test]
fn board() {
    use crate::github::{
        Window,
        board::{Detail, Section},
    };
    let account = r#"{"data":{"organization":{"id":"O_1","database":330236468,"login":"PerishLab","url":"https://github.com/PerishLab"}}}"#;
    let slice = r#"{"data":{"node":{"__typename":"ProjectV2","id":"PVT_1","number":1,"title":"Work","url":"https://github.com/orgs/PerishLab/projects/1","updatedAt":"2026-11-25T00:00:00Z","closed":false,"public":true,"readme":"","shortDescription":"","viewerCanUpdate":true,"owner":{"__typename":"Organization","id":"O_1","login":"PerishLab"},"items":{"totalCount":1,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"PVTI_1"}]}}}}"#;
    let meta = r#"{"data":{"node":{"id":"PVT_1","number":1,"title":"Work","url":"https://github.com/orgs/PerishLab/projects/1","updated":"2026-11-25T00:00:00Z","closed":false,"public":true,"writable":true,"description":"","readme":"","owner":{"kind":"Organization","id":"O_1","login":"PerishLab"}}}}"#;
    let subject = r#"{"data":{"node":{"id":"I_1","updated":"2026-11-25T00:00:00Z","repository":{"id":"R_1"}}}}"#;
    let candidates = r#"{"data":{"nodes":[{"__typename":"Issue","id":"I_1"}]}}"#;
    let seat = r#"{"data":{"node":{"__typename":"Repository","id":"R_1","nameWithOwner":"PerishLab/concord","owner":{"id":"O_1"}}}}"#;
    let exists = r#"{"data":{"node":{"__typename":"IssueFieldSingleSelect","id":"IF_1"}}}"#;
    let confirm = r#"{"data":{"node":{"__typename":"Issue","id":"I_1"}}}"#;
    let server = serve(vec![
        reply("200 OK", "", account),
        reply("200 OK", "", slice),
        reply("200 OK", "", meta),
        reply("200 OK", "", subject),
        reply("200 OK", "", candidates),
        reply("200 OK", "", seat),
        reply("200 OK", "", exists),
        reply("200 OK", "", confirm),
    ]);
    assert_eq!(
        server.client.account("PerishLab").unwrap().value.database,
        330236468
    );
    let window = Window::new(20, None).unwrap();
    let page = server
        .client
        .board("PVT_1", Section::Items(Detail::Observe), &window)
        .unwrap()
        .value;
    assert_eq!(page.project["id"], "PVT_1");
    assert_eq!(server.client.metadata("PVT_1").unwrap().value.number, 1);
    assert_eq!(
        server.client.subject("I_1").unwrap().value.repository.id,
        "R_1"
    );
    assert_eq!(
        server
            .client
            .candidates(&["I_1".into()])
            .unwrap()
            .value
            .len(),
        1
    );
    assert_eq!(server.client.seat("R_1").unwrap().value["id"], "R_1");
    assert_eq!(server.client.exists("IF_1").unwrap().value["id"], "IF_1");
    assert_eq!(server.client.confirm("I_1").unwrap().value["id"], "I_1");
    server.thread.join().unwrap();
}

#[test]
fn survey() {
    use crate::github::Window;
    let catalog = r#"{"data":{"organization":{"id":"O_1","login":"PerishLab","url":"https://github.com/PerishLab","repositories":{"totalCount":1,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"R_1","nameWithOwner":"PerishLab/concord","url":"https://github.com/PerishLab/concord","isArchived":false,"hasIssuesEnabled":true,"owner":{"id":"O_1","login":"PerishLab"}}]}}}}"#;
    let open = r#"{"data":{"node":{"id":"R_1","nameWithOwner":"PerishLab/concord","url":"https://github.com/PerishLab/concord","owner":{"id":"O_1","login":"PerishLab"},"issues":{"totalCount":1,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"I_1","number":1,"title":"A","url":"https://github.com/PerishLab/concord/issues/1","state":"OPEN","updatedAt":"2026-11-25T00:00:00Z","issueType":null,"repository":{"id":"R_1","nameWithOwner":"PerishLab/concord","owner":{"id":"O_1","login":"PerishLab"}}}]}}}}"#;
    let origins = r#"{"data":{"organization":{"id":"O_1","login":"PerishLab","issueFields":{"totalCount":1,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"__typename":"IssueFieldSingleSelect","id":"IF_1","fullDatabaseId":10,"name":"Phase","dataType":"SINGLE_SELECT","visibility":"ORG_ONLY","options":[]}]}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", catalog),
        reply("200 OK", "", open),
        reply("200 OK", "", origins),
    ]);
    let window = Window::new(100, None).unwrap();
    assert_eq!(
        server
            .client
            .catalog("PerishLab", &window)
            .unwrap()
            .value
            .owner["id"],
        "O_1"
    );
    assert_eq!(
        server
            .client
            .backlog("R_1", &window)
            .unwrap()
            .value
            .repository["id"],
        "R_1"
    );
    assert_eq!(
        server
            .client
            .origins("PerishLab", &window)
            .unwrap()
            .value
            .owner,
        "O_1"
    );
    server.thread.join().unwrap();
}
