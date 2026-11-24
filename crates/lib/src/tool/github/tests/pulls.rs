use super::{Cause, Outcome, reply, serve};
use crate::github::{Squash, Target};
use serde_json::json;

const HEAD: &str = "1234567890123456789012345678901234567890";
const MERGE: &str = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";

fn target() -> Target {
    Target::new("PerishLab", "runseal", 230).unwrap()
}

#[test]
fn pull() {
    let head = json!({"ref":"land/feature/228","sha":HEAD,"repo":null});
    let base = json!({"ref":"main","sha":HEAD,"repo":{"id":9,"full_name":"PerishLab/runseal"}});
    let body = json!({"id":12,"number":230,"node_id":"PR_fixture","html_url":"https://github.com/PerishLab/runseal/pull/230",
        "title":"fixture","body":null,"state":"closed","draft":false,"merged":true,"merge_commit_sha":MERGE,
        "head":head,"base":base}).to_string();
    let server = serve(vec![reply("200 OK", "", &body)]);
    let pull = server.client.pull(&target()).unwrap().value;
    assert_eq!(pull.head.sha, HEAD);
    assert!(pull.head.repo.is_none());
    assert_eq!(pull.merge.as_deref(), Some(MERGE));
    assert!(
        server.thread.join().unwrap()[0]
            .starts_with("GET /repos/PerishLab/runseal/pulls/230 HTTP/1.1")
    );
}

#[test]
fn matched() {
    let body = json!({"merged":true,"sha":MERGE}).to_string();
    let server = serve(vec![reply("200 OK", "", &body)]);
    let input = Squash::new(HEAD, Some("exact title"), Some("exact message")).unwrap();
    let receipt = server.client.squash(&target(), &input).unwrap();
    assert_eq!(receipt.value.sha, MERGE);
    let requests = server.thread.join().unwrap();
    assert!(requests[0].starts_with("PUT /repos/PerishLab/runseal/pulls/230/merge HTTP/1.1"));
    for field in [
        "\"merge_method\":\"squash\"",
        "\"commit_title\":\"exact title\"",
        "\"commit_message\":\"exact message\"",
    ] {
        assert!(requests[0].contains(field));
    }
    assert!(requests[0].contains(&format!("\"sha\":\"{HEAD}\"")));
}

#[test]
fn conflict() {
    assert!(Squash::new("abbreviated", None, None).is_err());
    let server = serve(vec![
        reply("409 Conflict", "", r#"{"message":"head moved"}"#),
        None,
    ]);
    let input = Squash::new(HEAD, None, None).unwrap();
    let error = server.client.squash(&target(), &input).unwrap_err();
    assert_eq!(
        (error.outcome, error.cause),
        (Outcome::Refused, Cause::Http)
    );
    assert_eq!(error.evidence.status, Some(409));
    let error = server.client.squash(&target(), &input).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert_eq!(error.evidence.attempts, 1);
    assert_eq!(server.thread.join().unwrap().len(), 2);
}

fn proposal() -> serde_json::Value {
    let origin = json!({"id":14,"full_name":"PerishLab/runseal"});
    let head = json!({"ref":"land/feature/228","sha":HEAD,"repo":origin});
    let base = json!({"ref":"main","sha":MERGE,"repo":origin});
    json!({"id":12,"number":230,"node_id":"PR_fixture","html_url":"https://github.com/PerishLab/runseal/pull/230",
        "title":"exact","body":"body","state":"open","draft":false,"merge_commit_sha":null,
        "head":head,"base":base})
}

#[test]
fn search() {
    use crate::github::{Compare, Search, State};

    let data = json!([proposal()]).to_string();
    let foreign = data.replace("\"ref\":\"main\"", "\"ref\":\"other\"");
    let server = serve(vec![
        reply("200 OK", "", &data),
        reply("200 OK", "", &foreign),
    ]);
    let comparison = Compare::new("land/feature/228", "main").unwrap();
    let search = Search::new(comparison, State::Open, 2, 3).unwrap();
    let pulls = server
        .client
        .pulls("PerishLab", "runseal", &search)
        .unwrap()
        .value;
    assert_eq!(pulls[0].number, 230);
    assert_eq!(pulls[0].merged, None);
    let error = server
        .client
        .pulls("PerishLab", "runseal", &search)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let calls = server.thread.join().unwrap();
    assert!(
        calls[0].contains(
            "head=PerishLab%3Aland%2Ffeature%2F228&base=main&state=open&per_page=3&page=2"
        )
    );
}

#[test]
fn create() {
    use crate::github::{Compare, Proposal};

    let data = proposal().to_string();
    let server = serve(vec![reply("201 Created", "", &data), None]);
    let comparison = Compare::new("land/feature/228", "main").unwrap();
    let proposal = Proposal::new(comparison, "exact", Some("body")).unwrap();
    let created = server
        .client
        .propose("PerishLab", "runseal", &proposal)
        .unwrap();
    assert_eq!(created.value.number, 230);
    let error = server
        .client
        .propose("PerishLab", "runseal", &proposal)
        .unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert_eq!(error.evidence.attempts, 1);
    let calls = server.thread.join().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].starts_with("POST /repos/PerishLab/runseal/pulls HTTP/1.1"));
    let payload = calls[0].lines().last().unwrap();
    let value: serde_json::Value = serde_json::from_str(payload).unwrap();
    assert_eq!(
        value,
        json!({"base":"main","head":"land/feature/228","title":"exact","body":"body"})
    );
}

#[test]
fn edit() {
    use crate::github::Edit;

    let good = proposal().to_string();
    let foreign = good.replace("PerishLab/runseal", "PerishLab/other");
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &foreign),
        None,
    ]);
    let edit = Edit::new("exact", "body").unwrap();
    let receipt = server.client.edit(&target(), &edit).unwrap();
    assert_eq!(receipt.value.number, 230);
    for cause in [Cause::Shape, Cause::Network] {
        let error = server.client.edit(&target(), &edit).unwrap_err();
        assert_eq!((error.outcome, error.cause), (Outcome::Unknown, cause));
        assert_eq!(error.evidence.attempts, 1);
    }
    let calls = server.thread.join().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("PATCH /repos/PerishLab/runseal/pulls/230 "));
    assert!(calls[0].contains("\"title\":\"exact\""));
}

#[test]
fn commit() {
    let good = json!({
        "sha":HEAD,"tree":{"sha":MERGE},"parents":[{"sha":MERGE}],
        "url":format!("https://api.github.com/repos/PerishLab/runseal/git/commits/{HEAD}")
    })
    .to_string();
    let foreign = good.replace("PerishLab/runseal", "PerishLab/other");
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &foreign),
    ]);
    let receipt = server.client.commit("PerishLab", "runseal", HEAD).unwrap();
    assert_eq!(receipt.value.tree.sha, MERGE);
    assert_eq!(receipt.value.parents[0].sha, MERGE);
    let error = server
        .client
        .commit("PerishLab", "runseal", HEAD)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains(&format!("/git/commits/{HEAD} ")));
}

#[test]
fn reference() {
    use crate::github::Reference;

    let found = json!({
        "ref":"refs/heads/feature/229",
        "url":"https://api.github.com/repos/PerishLab/runseal/git/refs/heads/feature/229",
        "object":{"sha":HEAD,"type":"commit"},
    })
    .to_string();
    let wrong = found.replace("feature/229", "feature/229-more");
    let server = serve(vec![
        reply("200 OK", "", &found),
        reply("200 OK", "", &wrong),
    ]);
    let selector = Reference::Branch("feature/229".into());
    let receipt = server
        .client
        .reference("PerishLab", "runseal", &selector)
        .unwrap();
    assert_eq!(receipt.value.object.sha, HEAD);
    let error = server
        .client
        .reference("PerishLab", "runseal", &selector)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(
        server.thread.join().unwrap()[0]
            .starts_with("GET /repos/PerishLab/runseal/git/ref/heads/feature/229 ")
    );
}

#[test]
fn readiness() {
    let data = json!({"id":"R_probe","nameWithOwner":"PerishLab/runseal",
        "pullRequest":{"id":"PR_fixture","number":230,"head":HEAD,"state":"BLOCKED"}});
    let good = json!({"data":{"repository":data}}).to_string();
    let bad = good.replace("PerishLab", "Foreign");
    let server = serve(vec![reply("200 OK", "", &good), reply("200 OK", "", &bad)]);
    assert_eq!(
        server.client.readiness(&target()).unwrap().value.state,
        "BLOCKED"
    );
    assert_eq!(
        server.client.readiness(&target()).unwrap_err().cause,
        Cause::Shape
    );
    assert!(server.thread.join().unwrap()[0].contains("mergeStateStatus"));
}

#[test]
fn revise() {
    use crate::github::lifecycle::Update;
    let mut edited = proposal();
    edited["body"] = json!("new");
    let mut closed = edited.clone();
    closed["state"] = json!("closed");
    closed["merged"] = json!(false);
    let server = serve(vec![
        reply("200 OK", "", &edited.to_string()),
        reply("200 OK", "", &closed.to_string()),
    ]);
    assert_eq!(
        server
            .client
            .revise(&target(), &Update::Body("new".into()))
            .unwrap()
            .value
            .title,
        "exact"
    );
    assert_eq!(
        server
            .client
            .revise(&target(), &Update::Close)
            .unwrap()
            .value
            .merged,
        Some(false)
    );
    let requests = server.thread.join().unwrap();
    assert!(requests[0].ends_with(r#"{"body":"new"}"#));
    assert!(requests[1].ends_with(r#"{"state":"closed"}"#));
}

#[test]
fn listings() {
    use crate::github::{Number, State, lifecycle::Selection};
    let mut pull = proposal();
    pull["base"]["ref"] = json!("next");
    let rows = json!([pull]).to_string();
    let more = "Link: <https://api.github.com/repositories/12/pulls?page=2>; rel=\"next\"\r\n";
    let server = serve(vec![
        reply("200 OK", more, &rows),
        reply("200 OK", "", &rows),
    ]);
    let read = |base| {
        let search = Selection::new(
            "land/feature/228",
            base,
            State::All,
            Number::new(1, 1).unwrap(),
        )
        .unwrap();
        server.client.listing("PerishLab", "runseal", &search)
    };
    assert_eq!(read(None).unwrap().value.next, Some(2));
    assert_eq!(read(Some("main")).unwrap_err().cause, Cause::Shape);
    let requests = server.thread.join().unwrap();
    assert!(!requests[0].lines().next().unwrap().contains("base="));
}
