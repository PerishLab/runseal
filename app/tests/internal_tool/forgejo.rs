#![cfg(unix)]

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
};

use tempfile::TempDir;

type Check = Box<dyn Fn(&str) + Send>;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[test]
fn tea() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("GET /api/v1/repos/PerishFire/demo "));
            assert!(request.contains("authorization: token tea-token"));
        }),
        200,
        r#"{"full_name":"PerishFire/demo"}"#,
    )]);
    let config = temp.path().join("tea.yml");
    std::fs::write(
        &config,
        format!("logins:\n- name: local\n  url: {base}\n  token: tea-token\n  active: false\n"),
    )
    .expect("Tea config should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_TEA_CONFIG", config)
        .env_remove("FORGEJO_TOKEN")
        .env_remove("FORGEJO_URL")
        .env_remove("GITHUB_SERVER_URL")
        .args([
            "@tool",
            "forgejo",
            "repo",
            "get",
            "--repo",
            "PerishFire/demo",
        ])
        .output()
        .expect("runseal should run");

    success(&output);
    handle.join().expect("mock server should finish");
}

#[test]
fn precedence() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| assert!(request.contains("authorization: token env-token"))),
        200,
        r#"{"id":1}"#,
    )]);

    let output = command(&temp, &base)
        .args(["@tool", "forgejo", "repo", "get", "--repo", "owner/repo"])
        .output()
        .expect("runseal should run");

    success(&output);
    handle.join().expect("mock server should finish");
}

#[test]
fn repo() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("POST /api/v1/orgs/PerishFire/repos "));
            assert!(request.contains(r#""name":"runseal""#));
            assert!(request.contains(r#""private":true"#));
            assert!(request.contains(r#""description":"operator runtime""#));
        }),
        201,
        r#"{"full_name":"PerishFire/runseal"}"#,
    )]);

    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "repo",
            "create",
            "--owner",
            "PerishFire",
            "--name",
            "runseal",
            "--private",
            "true",
            "--description",
            "operator runtime",
        ])
        .output()
        .expect("runseal should run");

    success(&output);
    handle.join().expect("mock server should finish");
}

#[test]
fn pull() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("POST /api/v1/repos/owner/repo/pulls "));
            assert!(request.contains(r#""head":"topic""#));
            assert!(request.contains(r#""base":"main""#));
            assert!(request.contains(r#""title":"change""#));
        }),
        201,
        r#"{"number":7}"#,
    )]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "pr",
            "create",
            "--repo",
            "owner/repo",
            "--head",
            "topic",
            "--base",
            "main",
            "--title",
            "change",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");

    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("POST /api/v1/repos/owner/repo/pulls/7/merge "));
            assert!(request.contains(r#""Do":"squash""#));
            assert!(request.contains(r#""delete_branch_after_merge":true"#));
            assert!(request.contains(r#""head_commit_id":"abc123""#));
        }),
        200,
        r#"{"merged":true}"#,
    )]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "pr",
            "merge",
            "--repo",
            "owner/repo",
            "--number",
            "7",
            "--head",
            "abc123",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");
}

#[test]
fn actions() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(
                request.starts_with("PUT /api/v1/repos/owner/repo/actions/secrets/RELEASE_KEY ")
            );
            assert!(request.contains(r#""data":"secret-value""#));
        }),
        204,
        "",
    )]);
    let output = command(&temp, &base)
        .env("RELEASE_KEY", "secret-value")
        .args([
            "@tool",
            "forgejo",
            "secret",
            "upsert",
            "--repo",
            "owner/repo",
            "--name",
            "RELEASE_KEY",
            "--value-env",
            "RELEASE_KEY",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");

    let (base, handle) = mock(vec![
        (
            Box::new(|request| {
                assert!(
                    request
                        .starts_with("GET /api/v1/repos/owner/repo/actions/variables/PUBLIC_URL ")
                );
            }),
            404,
            r#"{"message":"not found"}"#,
        ),
        (
            Box::new(|request| {
                assert!(
                    request
                        .starts_with("POST /api/v1/repos/owner/repo/actions/variables/PUBLIC_URL ")
                );
                assert!(request.contains(r#""value":"https://releases.test""#));
            }),
            201,
            r#"{"name":"PUBLIC_URL"}"#,
        ),
    ]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "variable",
            "upsert",
            "--repo",
            "owner/repo",
            "--name",
            "PUBLIC_URL",
            "--value",
            "https://releases.test",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");

    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with(
                "POST /api/v1/repos/owner/repo/actions/workflows/release-beta.yml/dispatches "
            ));
            assert!(request.contains(r#""ref":"main""#));
            assert!(request.contains(r#""version_override":"v1""#));
            assert!(request.contains(r#""return_run_info":true"#));
        }),
        201,
        r#"{"id":44,"run_number":9}"#,
    )]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "workflow",
            "dispatch",
            "--repo",
            "owner/repo",
            "--workflow",
            "release-beta.yml",
            "--ref",
            "main",
            "--input",
            "version_override=v1",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");

    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("GET /api/v1/repos/owner/repo/actions/runs?"));
            assert!(request.contains("limit=1"));
        }),
        200,
        r#"{"total_count":3,"workflow_runs":[{"id":3},{"id":2},{"id":1}]}"#,
    )]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "run",
            "list",
            "--repo",
            "owner/repo",
            "--limit",
            "1",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["workflow_runs"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["workflow_runs"][0]["id"], 3);
    handle.join().expect("mock server should finish");
}

#[test]
fn guard() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![
        (
            Box::new(|request| {
                assert!(request.starts_with("GET /api/v1/repos/owner/repo/pulls/7 "));
            }),
            200,
            r#"{"head":{"sha":"abc123"}}"#,
        ),
        (
            Box::new(|request| {
                assert!(request.starts_with("GET /api/v1/repos/owner/repo/actions/runs?"));
                assert!(request.contains("head_sha=abc123"));
                assert!(request.contains("workflow_id=guard.yml"));
                assert!(!request.contains("event=pull_request"));
            }),
            200,
            r#"{"workflow_runs":[{"id":9,"status":"success","commit_sha":"other","workflow_id":"guard.yml","trigger_event":"push"},{"id":8,"status":"success","commit_sha":"abc123","workflow_id":"guard.yml","trigger_event":"pull_request"}]}"#,
        ),
    ]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "pr",
            "guard",
            "--repo",
            "owner/repo",
            "--number",
            "7",
            "--interval",
            "0",
            "--timeout",
            "1",
        ])
        .output()
        .expect("runseal should run");

    success(&output);
    handle.join().expect("mock server should finish");
}

#[test]
fn watch() {
    let temp = TempDir::new().expect("temp dir should be created");
    let (base, handle) = mock(vec![(
        Box::new(|request| {
            assert!(request.starts_with("GET /api/v1/repos/owner/repo/actions/runs/8 "));
        }),
        200,
        r#"{"id":8,"status":"success"}"#,
    )]);
    let output = command(&temp, &base)
        .args([
            "@tool",
            "forgejo",
            "run",
            "watch",
            "--repo",
            "owner/repo",
            "--id",
            "8",
            "--interval",
            "0",
            "--timeout",
            "1",
        ])
        .output()
        .expect("runseal should run");
    success(&output);
    handle.join().expect("mock server should finish");

    let output = command(&temp, "http://127.0.0.1:1")
        .args(["@tool", "forgejo", "run", "cancel"])
        .output()
        .expect("runseal should run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Forgejo v15 has no supported workflow cancel API")
    );
}

fn command(temp: &TempDir, base: &str) -> Command {
    let mut command = bin();
    command
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_FORGEJO_API_BASE", base)
        .env("FORGEJO_TOKEN", "env-token")
        .env_remove("FORGEJO_URL")
        .env_remove("GITHUB_SERVER_URL");
    command
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn mock(steps: Vec<(Check, u16, &'static str)>) -> (String, thread::JoinHandle<()>) {
    let server = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
    let address = server
        .local_addr()
        .expect("mock server address should exist");
    let handle = thread::spawn(move || {
        for (check, status, body) in steps {
            let mut stream = server.accept().expect("mock request should arrive").0;
            let mut buffer = [0_u8; 16384];
            let read = stream
                .read(&mut buffer)
                .expect("request should be readable");
            let request = String::from_utf8_lossy(&buffer[..read]);
            check(&request);
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
            .expect("response should be written");
        }
    });
    (format!("http://{address}"), handle)
}
