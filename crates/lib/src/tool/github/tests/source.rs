use super::{reply, serve};
use crate::github::{Cause, Outcome};
use serde_json::json;

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BLOB: &str = "ce013625030ba8dba906f756967f9e9ca394464a";

#[test]
fn blob() {
    let body = json!({
        "sha":BLOB, "url":format!("https://api.github.com/repos/PerishLab/runseal/git/blobs/{BLOB}"),
        "size":6, "encoding":"base64", "content":"aGVsbG8K\n",
    }).to_string();
    let corrupt = body.replace("aGVsbG8K", "eGVsbG8K");
    let server = serve(vec![
        reply("200 OK", "", &body),
        reply("200 OK", "", &corrupt),
    ]);
    let found = server.client.blob("PerishLab", "runseal", BLOB).unwrap();
    assert_eq!(found.value.bytes, b"hello\n");
    let error = server
        .client
        .blob("PerishLab", "runseal", BLOB)
        .unwrap_err();
    assert_eq!(
        (error.outcome, error.cause),
        (Outcome::Unknown, Cause::Shape)
    );
    assert!(server.thread.join().unwrap()[0].contains(&format!("/git/blobs/{BLOB} ")));
}

#[test]
fn tree() {
    let entry = json!({"path":"dir/name","mode":"100644","type":"blob","sha":BLOB,"size":6});
    let body = json!({
        "sha":HEAD,"url":format!("https://api.github.com/repos/PerishLab/runseal/git/trees/{HEAD}"),
        "truncated":true,"tree":[entry],
    })
    .to_string();
    let duplicate = json!({
        "sha":HEAD,"url":format!("https://api.github.com/repos/PerishLab/runseal/git/trees/{HEAD}"),
        "truncated":false,"tree":[entry,entry],
    })
    .to_string();
    let server = serve(vec![
        reply("200 OK", "", &body),
        reply("200 OK", "", &duplicate),
    ]);
    let found = server
        .client
        .tree("PerishLab", "runseal", HEAD, true)
        .unwrap();
    assert!(found.value.truncated);
    assert_eq!(found.value.rows[0].path, "dir/name");
    let error = server
        .client
        .tree("PerishLab", "runseal", HEAD, true)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains(&format!("/git/trees/{HEAD}?recursive=1 ")));
}

#[test]
fn contents() {
    let encoded = "docs/a%20b%23.txt";
    let path = format!("/repos/PerishLab/runseal/contents/{encoded}?ref={HEAD}");
    let body = json!({
        "path":"docs/a b#.txt","type":"file","url":format!("https://api.github.com{path}"),
        "sha":BLOB,"size":6,"encoding":"base64","content":"aGVsbG8K\n",
    })
    .to_string();
    let moved = body.replace(HEAD, &"b".repeat(40));
    let server = serve(vec![
        reply("200 OK", "", &body),
        reply("200 OK", "", &moved),
    ]);
    let found = server
        .client
        .contents("PerishLab", "runseal", HEAD, "docs/a b#.txt")
        .unwrap();
    assert_eq!(found.value.blob.bytes, b"hello\n");
    let error = server
        .client
        .contents("PerishLab", "runseal", HEAD, "docs/a b#.txt")
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].starts_with(&format!("GET {path} ")));
}

#[test]
fn branch() {
    let good = json!({"name":"main","protected":true,"commit":{
        "sha":HEAD,"url":format!("https://api.github.com/repos/PerishLab/runseal/commits/{HEAD}"),
    }})
    .to_string();
    let foreign = good.replace("PerishLab/runseal", "PerishLab/wharf");
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &foreign),
    ]);
    let receipt = server
        .client
        .branch("PerishLab", "runseal", "main")
        .unwrap();
    assert!(receipt.value.protected);
    let error = server
        .client
        .branch("PerishLab", "runseal", "main")
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(
        server.thread.join().unwrap()[0].starts_with("GET /repos/PerishLab/runseal/branches/main ")
    );
}

#[test]
fn permission() {
    let body = r#"{"permission":"write","role_name":"custom","user":{"id":12,"node_id":"U_user","type":"User","login":"reviewer"}}"#;
    let wrong = body.replace("reviewer", "other");
    let server = serve(vec![reply("200 OK", "", body), reply("200 OK", "", &wrong)]);
    let receipt = server
        .client
        .permission("PerishLab", "runseal", "reviewer")
        .unwrap();
    assert_eq!(receipt.value.role, "custom");
    assert_eq!(receipt.value.user.id, 12);
    let error = server
        .client
        .permission("PerishLab", "runseal", "reviewer")
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(
        server.thread.join().unwrap()[0]
            .starts_with("GET /repos/PerishLab/runseal/collaborators/reviewer/permission ")
    );
}

#[test]
fn matching() {
    let body = json!([
        {"ref":"refs/heads/auto/12","url":"https://api.github.com/repos/PerishLab/runseal/git/refs/heads/auto/12","object":{"sha":HEAD,"type":"commit"}},
        {"ref":"refs/heads/auto/123","url":"https://api.github.com/repos/PerishLab/runseal/git/refs/heads/auto/123","object":{"sha":HEAD,"type":"commit"}}
    ]).to_string();
    let foreign = body.replace(
        "PerishLab/runseal/git/refs/heads/auto/123",
        "PerishLab/wharf/git/refs/heads/auto/123",
    );
    let server = serve(vec![
        reply("200 OK", "", &body),
        reply("200 OK", "", &foreign),
        reply(
            "200 OK",
            "Link: <https://api.github.com/next>; rel=\"next\"\r\n",
            "[]",
        ),
        reply("200 OK", "", "[]"),
    ]);
    let branches = server
        .client
        .matching("PerishLab", "runseal", "auto/12")
        .unwrap();
    assert_eq!(branches.value.len(), 2);
    assert_eq!(branches.value[1].reference, "refs/heads/auto/123");
    for _ in 0..2 {
        let error = server
            .client
            .matching("PerishLab", "runseal", "auto/12")
            .unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
    }
    assert!(
        server
            .client
            .matching("PerishLab", "runseal", "auto/12")
            .unwrap()
            .value
            .is_empty()
    );
    assert!(
        server.thread.join().unwrap()[0]
            .starts_with("GET /repos/PerishLab/runseal/git/matching-refs/heads/auto/12 ")
    );
}
