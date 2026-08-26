mod support;

use support::{Seat, serve, text};

fn pass(args: &[&str], status: &str, body: &str) -> (String, Vec<String>) {
    let (url, handle) = serve(status, body, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(args);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    (text(&output.stdout), seen)
}

#[test]
fn opened() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "pull",
            "show",
            "PerishFire/runseal#71",
        ],
        "200 OK",
        r#"{"number":71,"state":"open","title":"verbs"}"#,
    );
    assert_eq!(out.trim(), "71\topen\tverbs");
    assert!(
        seen[0].starts_with("GET /api/v1/repos/PerishFire/runseal/pulls/71 "),
        "{}",
        seen[0]
    );
}

#[test]
fn raised() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "pull",
            "create",
            "--title",
            "pack",
            "--head",
            "topic",
        ],
        "201 Created",
        r#"{"number":8,"state":"open","title":"pack"}"#,
    );
    assert_eq!(out.trim(), "8\topen\tpack");
    assert!(
        seen[0].starts_with("POST /api/v1/repos/PerishFire/runseal/pulls "),
        "{}",
        seen[0]
    );
}

#[test]
fn merged() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "pull",
            "merge",
            "71",
            "--head",
            "abc",
        ],
        "204 No Content",
        "",
    );
    assert_eq!(out.trim(), "ok");
    assert!(
        seen[0].starts_with("POST /api/v1/repos/PerishFire/runseal/pulls/71/merge "),
        "{}",
        seen[0]
    );
}

#[test]
fn standing() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "status",
            "show",
            "abc",
        ],
        "200 OK",
        r#"{"state":"success","statuses":[]}"#,
    );
    assert_eq!(out.trim(), "success");
    assert!(
        seen[0].starts_with("GET /api/v1/repos/PerishFire/runseal/commits/abc/status "),
        "{}",
        seen[0]
    );
}

#[test]
fn stem() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "branch",
            "show",
            "main",
        ],
        "200 OK",
        r#"{"name":"main"}"#,
    );
    assert_eq!(out.trim(), "main");
    assert!(
        seen[0].starts_with("GET /api/v1/repos/PerishFire/runseal/branches/main "),
        "{}",
        seen[0]
    );
}

#[test]
fn stored() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "secret",
            "set",
            "TOKEN",
            "--body",
            "held",
        ],
        "204 No Content",
        "",
    );
    assert_eq!(out.trim(), "ok");
    assert!(
        seen[0].starts_with("PUT /api/v1/repos/PerishFire/runseal/actions/secrets/TOKEN "),
        "{}",
        seen[0]
    );
}

#[test]
fn sent() {
    let (out, seen) = pass(
        &[
            ":perish",
            "@forgejo",
            "--repo",
            "PerishFire/runseal",
            "workflow",
            "dispatch",
            "release.yml",
            "--ref",
            "main",
        ],
        "204 No Content",
        "",
    );
    assert_eq!(out.trim(), "ok");
    assert!(
        seen[0].starts_with(
            "POST /api/v1/repos/PerishFire/runseal/actions/workflows/release.yml/dispatches "
        ),
        "{}",
        seen[0]
    );
}

#[test]
fn fetch() {
    let (url, handle) = support::serve("200 OK", r#"{"ok":true}"#, 1);
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--json",
        "get",
        &format!("{url}/stable"),
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains(r#""version":1"#) || stdout.contains(r#""version": 1"#));
    assert!(stdout.contains(r#""ok":true"#) || stdout.contains(r#""ok": true"#));
}

#[test]
fn creates() {
    let (url, handle) = serve("201 Created", r#"{"name":"portfolio"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/portfolio",
        "repo",
        "create",
        "--body",
        r#"{"private":false}"#,
    ]);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        seen[0].starts_with("POST /api/v1/orgs/PerishLab/repos "),
        "{}",
        seen[0]
    );
    assert!(seen[0].contains(r#""name":"portfolio""#), "{}", seen[0]);
    assert!(seen[0].contains(r#""private":false"#), "{}", seen[0]);
}

#[test]
fn mismatch() {
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/portfolio",
        "repo",
        "create",
        "--body",
        r#"{"name":"another"}"#,
    ]);
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("name must match --repo"),
        "{}",
        text(&output.stderr)
    );
}
