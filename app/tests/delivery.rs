mod support;

use std::collections::BTreeMap;

use serde_json::json;
use support::{Seat, sequence, serve, text};

struct Case {
    args: &'static [&'static str],
    status: &'static str,
    body: &'static str,
    route: &'static str,
}

#[test]
fn writes() {
    let cases = [
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "pull",
                "edit",
                "7",
                "--title",
                "new",
            ],
            status: "200 OK",
            body: r#"{"number":7,"title":"new"}"#,
            route: "PATCH /api/v1/repos/PerishFire/runseal/pulls/7 ",
        },
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "branch",
                "create",
                "topic",
                "--from",
                "main",
            ],
            status: "201 Created",
            body: r#"{"name":"topic"}"#,
            route: "POST /api/v1/repos/PerishFire/runseal/branches ",
        },
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "protection",
                "create",
                "--body",
                "{}",
            ],
            status: "201 Created",
            body: r#"{"rule_name":"main"}"#,
            route: "POST /api/v1/repos/PerishFire/runseal/branch_protections ",
        },
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "protection",
                "edit",
                "main",
                "--body",
                "{}",
            ],
            status: "200 OK",
            body: r#"{"rule_name":"main"}"#,
            route: "PATCH /api/v1/repos/PerishFire/runseal/branch_protections/main ",
        },
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "repo",
                "edit",
                "--body",
                "{}",
            ],
            status: "200 OK",
            body: r#"{"name":"runseal"}"#,
            route: "PATCH /api/v1/repos/PerishFire/runseal ",
        },
        Case {
            args: &["--repo", "PerishFire/runseal", "repo", "delete"],
            status: "204 No Content",
            body: "",
            route: "DELETE /api/v1/repos/PerishFire/runseal ",
        },
        Case {
            args: &["--repo", "PerishFire/runseal", "secret", "delete", "TOKEN"],
            status: "204 No Content",
            body: "",
            route: "DELETE /api/v1/repos/PerishFire/runseal/actions/secrets/TOKEN ",
        },
        Case {
            args: &[
                "--repo",
                "PerishFire/runseal",
                "review",
                "create",
                "7",
                "--body",
                "ok",
            ],
            status: "200 OK",
            body: r#"{"id":8,"body":"ok"}"#,
            route: "POST /api/v1/repos/PerishFire/runseal/pulls/7/reviews ",
        },
    ];
    for case in cases {
        check(case);
    }
}

#[test]
fn perception() {
    let cases = [
        Case {
            args: &["--repo", "PerishFire/runseal", "run", "show", "9"],
            status: "200 OK",
            body: r#"{"id":9,"status":"success"}"#,
            route: "GET /api/v1/repos/PerishFire/runseal/actions/runs/9 ",
        },
        Case {
            args: &["--repo", "PerishFire/runseal", "job", "log", "9", "2"],
            status: "200 OK",
            body: "held log",
            route: "GET /PerishFire/runseal/actions/runs/9/jobs/2/attempt/1/logs ",
        },
        Case {
            args: &["--repo", "PerishFire/runseal", "review", "list", "7"],
            status: "200 OK",
            body: "[]",
            route: "GET /api/v1/repos/PerishFire/runseal/pulls/7/reviews?limit=50&page=1 ",
        },
        Case {
            args: &["--repo", "PerishFire/runseal", "label", "list"],
            status: "200 OK",
            body: "[]",
            route: "GET /api/v1/repos/PerishFire/runseal/labels?limit=50&page=1 ",
        },
    ];
    for case in cases {
        check(case);
    }
}

#[test]
fn structured() {
    let (url, handle) = serve(
        "200 OK",
        r#"{"total_count":2,"workflow_runs":[{"run_number":4},{"run_number":7}]}"#,
        1,
    );
    let vars = BTreeMap::from([
        ("FORGEJO_URL".into(), url),
        ("FORGEJO_TOKEN".into(), "secret-token".into()),
    ]);
    let args = words(&["--repo", "PerishFire/runseal", "task", "list", "7"]);
    let reply = runseal::tool::call("forgejo", &args, &vars).expect("structured call");
    handle.join().expect("server");
    assert_eq!(reply.kind, "tasks");
    assert_eq!(reply.value, json!([{"run_number": 7}]));
}

#[test]
fn paged() {
    let rows = (0..50)
        .map(|number| json!({"run_number": number}))
        .collect::<Vec<_>>();
    let body = json!({"total_count": 51, "workflow_runs": rows}).to_string();
    let (url, handle) = serve("200 OK", &body, 2);
    let vars = BTreeMap::from([
        ("FORGEJO_URL".into(), url),
        ("FORGEJO_TOKEN".into(), "secret-token".into()),
    ]);
    let args = words(&["--repo", "PerishFire/runseal", "task", "list", "7"]);
    let reply = runseal::tool::call("forgejo", &args, &vars).expect("paged call");
    let seen = handle.join().expect("server");
    assert_eq!(reply.value.as_array().map(Vec::len), Some(2));
    assert!(seen[1].contains("page=2"), "{}", seen[1]);
}

#[test]
fn follows() {
    let running = job("running", false);
    let success = job("success", true);
    let (url, handle) = sequence(vec![
        ("200 OK".into(), running.to_string()),
        ("200 OK".into(), "one\n".into()),
        ("200 OK".into(), success.to_string()),
        ("200 OK".into(), "one\ntwo\n".into()),
    ]);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishFire/runseal",
        "job",
        "log",
        "9",
        "0",
        "--watch",
        "--poll-ms",
        "0",
        "--timeout-ms",
        "1000",
    ]);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "one\ntwo\n");
    assert!(seen[0].starts_with("POST /PerishFire/runseal/actions/runs/9/jobs/0/attempt/1 "));
    assert!(seen[1].starts_with("GET /PerishFire/runseal/actions/runs/9/jobs/0/attempt/1/logs "));
}

#[test]
fn fails() {
    let (url, handle) = sequence(vec![
        ("200 OK".into(), job("failure", true).to_string()),
        ("200 OK".into(), "held log\n".into()),
    ]);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishFire/runseal",
        "job",
        "log",
        "9",
        "0",
        "--watch",
    ]);
    handle.join().expect("server");
    assert!(!output.status.success());
    assert_eq!(text(&output.stdout), "held log\n");
    assert!(text(&output.stderr).contains("ended with failure"));
}

fn job(status: &str, done: bool) -> serde_json::Value {
    let jobs = vec![json!({"status": status})];
    json!({"state": {"run": {"done": done, "jobs": jobs}}})
}

#[test]
fn jobs() {
    let body = r#"{"state":{"run":{"jobs":[{"name":"resolve","status":"success"},{"name":"build","status":"failure"}]}}}"#;
    let (url, handle) = serve("200 OK", body, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishFire/runseal",
        "job",
        "list",
        "261",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "0\tsuccess\tresolve\n1\tfailure\tbuild\n"
    );
}

#[test]
fn help() {
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[":perish", "@forgejo", "--help"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let shown = text(&output.stdout);
    assert!(shown.contains("RESOURCE VERB"), "{shown}");
    assert!(shown.contains("task list"), "{shown}");
}

fn check(case: Case) {
    let (url, handle) = serve(case.status, case.body, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[&[":perish", "@forgejo"], case.args].concat());
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(seen[0].starts_with(case.route), "{}", seen[0]);
}

fn words(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}
