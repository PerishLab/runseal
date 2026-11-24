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
