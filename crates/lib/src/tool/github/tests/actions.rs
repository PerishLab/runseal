use super::{Cause, Outcome, reply, serve};
use crate::github::{Dispatch, Listing};
use serde_json::json;
use std::collections::BTreeMap;

const HEAD: &str = "1234567890123456789012345678901234567890";
const ACCEPTED: &str = r#"{"workflow_run_id":21,"run_url":"https://api.github.com/repos/PerishLab/wharf/actions/runs/21","html_url":"https://github.com/PerishLab/wharf/actions/runs/21"}"#;

#[test]
fn launch() {
    let inputs = BTreeMap::from([("marker".into(), "v0.53.0-rc.1".into())]);
    let request = Dispatch::new("manager.yml", "main", inputs).unwrap();
    let server = serve(vec![reply("200 OK", "", ACCEPTED)]);
    let receipt = server
        .client
        .dispatch("perishlab", "wharf", &request)
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    assert_eq!(receipt.value.id, 21);
    let calls = server.thread.join().unwrap();
    assert!(calls[0].starts_with(
        "POST /repos/perishlab/wharf/actions/workflows/manager.yml/dispatches HTTP/1.1"
    ));
    assert!(
        calls[0]
            .to_ascii_lowercase()
            .contains("x-github-api-version: 2026-03-10")
    );
    assert!(calls[0].contains("\"marker\":\"v0.53.0-rc.1\""));
}

#[test]
fn unknown() {
    let request = Dispatch::new("manager.yml", "main", BTreeMap::new()).unwrap();
    let foreign = ACCEPTED.replace("PerishLab/wharf", "PerishLab/other");
    let server = serve(vec![
        reply("204 No Content", "", ""),
        reply("200 OK", "", &foreign),
        None,
    ]);
    for cause in [Cause::Shape, Cause::Shape, Cause::Network] {
        let error = server
            .client
            .dispatch("PerishLab", "wharf", &request)
            .unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert_eq!(error.cause, cause);
        assert_eq!(error.evidence.attempts, 1);
    }
    assert_eq!(server.thread.join().unwrap().len(), 3);
}

#[test]
fn listing() {
    let row = json!({"id":7,"run_id":21,"run_attempt":2,"head_sha":HEAD,
    "html_url":"https://github.com/PerishLab/wharf/actions/runs/21/job/7",
    "name":"proof","status":"completed","conclusion":"success","steps":[
        {"name":"native","status":"completed","conclusion":"success","number":1}
    ]});
    let good = json!({"total_count":2,"jobs":[row]}).to_string();
    let wrong = good.replace("\"run_attempt\":2", "\"run_attempt\":3");
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &wrong),
    ]);
    let query = Listing::new(21, 2, 2, 1).unwrap();
    let receipt = server.client.jobs("PerishLab", "wharf", query).unwrap();
    assert_eq!(
        receipt.value.rows[0].steps.as_ref().unwrap()[0].name,
        "native"
    );
    let error = server.client.jobs("PerishLab", "wharf", query).unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(
        server.thread.join().unwrap()[0]
            .contains("/actions/runs/21/attempts/2/jobs?per_page=1&page=2")
    );
}

#[test]
fn release() {
    let asset = r#"{"id":77,"node_id":"RA_file","name":"pnpm-win32-x64.zip","size":123,"state":"uploaded","content_type":"application/zip","digest":null,"url":"https://api.github.com/repos/pnpm/pnpm/releases/assets/77","browser_download_url":"https://github.com/pnpm/pnpm/releases/download/v10.1.0/pnpm-win32-x64.zip"}"#;
    let release = format!(
        r#"{{"id":22,"node_id":"RE_demo","tag_name":"v10.1.0","name":"pnpm","draft":false,"prerelease":false,"url":"https://api.github.com/repos/pnpm/pnpm/releases/22","assets":[{asset}]}}"#
    );
    let mismatched = release.replace("/download/v10.1.0/", "/download/v10.2.0/");
    let foreign = asset.replace(
        "https://github.com/pnpm/pnpm",
        "https://example.invalid/pnpm/pnpm",
    );
    let server = serve(vec![
        reply("200 OK", "", &release),
        reply("200 OK", "", &mismatched),
        reply("200 OK", "", asset),
        reply("200 OK", "", &foreign),
    ]);
    let observed = server.client.release("pnpm", "pnpm", "v10.1.0").unwrap();
    assert_eq!(observed.value.assets[0].id, 77);
    assert!(observed.value.assets[0].digest.is_none());
    assert_eq!(
        server
            .client
            .release("pnpm", "pnpm", "v10.1.0")
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    assert_eq!(
        server.client.asset("pnpm", "pnpm", 77).unwrap().value.size,
        123
    );
    assert_eq!(
        server.client.asset("pnpm", "pnpm", 77).unwrap_err().cause,
        Cause::Shape
    );
    let requests = server.thread.join().unwrap();
    assert!(requests[0].starts_with("GET /repos/pnpm/pnpm/releases/tags/v10.1.0 "));
    assert!(requests[2].starts_with("GET /repos/pnpm/pnpm/releases/assets/77 "));
}

const CHECK: &str = "1234567890123456789012345678901234567890";

#[test]
fn checks() {
    use crate::github::Filter;

    let first = json!({"id":2,"name":"Guard","head_sha":CHECK,"status":"completed","conclusion":"success",
        "details_url":"https://github.com/PerishLab/runseal/actions/runs/21","app":{"id":4,"slug":"github-actions"}});
    let one = json!({"total_count":2,"check_runs":[first]}).to_string();
    let wrong = json!({"id":3,"name":"Guard","head_sha":"abcdefabcdefabcdefabcdefabcdefabcdefabcd","status":"completed","conclusion":"success"});
    let two = json!({"total_count":2,"check_runs":[wrong]}).to_string();
    let server = serve(vec![reply("200 OK", "", &one), reply("200 OK", "", &two)]);
    let filter = Filter::new(CHECK, 2, 1).unwrap();
    let result = server
        .client
        .checks("PerishLab", "runseal", &filter)
        .unwrap();
    assert_eq!(result.value.total, 2);
    assert_eq!(result.value.rows[0].app.as_ref().unwrap().id, 4);
    let error = server
        .client
        .checks("PerishLab", "runseal", &filter)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains("/check-runs?per_page=1&page=2"));
}

#[test]
fn workflow() {
    use crate::github::Invocation;

    let repository = json!({"id":8,"full_name":"PerishLab/runseal"});
    let body = json!({"id":21,"workflow_id":22,"run_attempt":2,"head_sha":CHECK,
        "html_url":"https://github.com/PerishLab/runseal/actions/runs/21","event":"pull_request",
        "path":".github/workflows/guard.yml","status":"completed","conclusion":"success",
        "repository":repository})
    .to_string();
    let server = serve(vec![reply("200 OK", "", &body), reply("200 OK", "", &body)]);
    let request = Invocation::new(21, Some(2)).unwrap();
    let result = server
        .client
        .workflow("PerishLab", "runseal", request)
        .unwrap();
    assert_eq!(result.value.attempt, 2);
    let wrong = Invocation::new(21, Some(3)).unwrap();
    let error = server
        .client
        .workflow("PerishLab", "runseal", wrong)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("/actions/runs/21/attempts/2"));
    assert!(requests[1].contains("/actions/runs/21/attempts/3"));
}
