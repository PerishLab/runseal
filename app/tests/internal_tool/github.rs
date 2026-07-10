#![cfg(unix)]

use std::{
    ffi::OsString,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[test]
fn issue_comment_write() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("repo");
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    write_git_stub(
        &bin_dir.join("git"),
        "git@gitee.com:perishme/perish.top.git",
        "feat/prefix",
    );

    let body_file = cwd.join("body.md");
    std::fs::write(&body_file, "Hello from file\n").expect("body file should be written");
    let token_file = cwd.join("github.env");
    std::fs::write(&token_file, "GITHUB_TOKEN=file-token\n").expect("token file should be written");

    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("POST /repos/PerishCode/runseal/issues/46/comments "));
            assert!(request.contains("authorization: Bearer file-token"));
            assert!(request.contains(
                r#"Requested-By-Repo: perishme/perish.top\nRequested-By-Branch: feat/prefix\n\nHello from file\n"#
            ));
        },
        r#"{"id":46,"html_url":"https://github.test/comment/46"}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .args([
            "@tool",
            "github",
            "issue",
            "comment",
            "create",
            "--repo",
            "PerishCode/runseal",
            "--number",
            "46",
            "--body-file",
            body_file.to_str().unwrap(),
            "--prefix-enable=true",
            "--token-file",
            token_file.to_str().unwrap(),
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(payload["id"], 46);

    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("PATCH /repos/perishme/perish.top/issues/77 "));
            assert!(request.contains("authorization: Bearer explicit-token"));
            assert!(request.contains(r#""body":"Already prefixed""#));
        },
        r#"{"number":77,"html_url":"https://github.test/issues/77"}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "body",
            "update",
            "--repo",
            "perishme/perish.top",
            "--number",
            "77",
            "--body",
            "Already prefixed",
            "--prefix-enable=true",
            "--token",
            "explicit-token",
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");
}

#[test]
fn issue_create() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("repo");
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    write_git_stub(
        &bin_dir.join("git"),
        "git@gitee.com:perishme/perish.top.git",
        "feat/prefix",
    );

    let body_file = cwd.join("body.md");
    std::fs::write(&body_file, "Issue body\n").expect("body file should be written");
    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("POST /repos/PerishCode/runseal/issues "));
            assert!(request.contains("authorization: Bearer env-token"));
            assert!(request.contains(r#""title":"Open one issue""#));
            assert!(request.contains(
                r#"Requested-By-Repo: perishme/perish.top\nRequested-By-Branch: feat/prefix\n\nIssue body\n"#
            ));
        },
        r#"{"number":88,"html_url":"https://github.test/issues/88"}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "create",
            "--repo",
            "PerishCode/runseal",
            "--title",
            "Open one issue",
            "--body-file",
            body_file.to_str().unwrap(),
            "--prefix-enable=true",
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(payload["number"], 88);
}

#[test]
fn issue_prefix_rules() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("repo");
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    write_git_stub(&bin_dir.join("git"), "", "feat/prefix");

    let failed = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "comment",
            "create",
            "--repo",
            "PerishCode/runseal",
            "--number",
            "46",
            "--body",
            "Hello",
            "--prefix-enable",
        ])
        .output()
        .expect("runseal should run");
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("cannot parse owner/repo from origin url"),
        "stderr: {}",
        String::from_utf8_lossy(&failed.stderr)
    );

    write_git_stub(
        &bin_dir.join("git"),
        "git@gitee.com:perishme/perish.top.git",
        "feat/prefix",
    );
    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("POST /repos/PerishCode/runseal/issues/12/comments "));
            assert!(request.contains(r#""body":"Hello""#));
            assert!(!request.contains("Requested-By-Repo:"));
        },
        r#"{"id":12}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "comment",
            "create",
            "--repo",
            "PerishCode/runseal",
            "--number",
            "12",
            "--body",
            "Hello",
            "--prefix-enable=false",
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");
}

#[test]
fn issue_body_max() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("repo");
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    write_git_stub(
        &bin_dir.join("git"),
        "git@gitee.com:perishme/perish.top.git",
        "feat/prefix",
    );

    let too_long = "x".repeat(101);
    let failed = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "comment",
            "create",
            "--repo",
            "example/demo",
            "--number",
            "12",
            "--body",
            &too_long,
        ])
        .output()
        .expect("runseal should run");
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("body length 101 exceeds --body-max=100"),
        "stderr: {}",
        String::from_utf8_lossy(&failed.stderr)
    );

    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("POST /repos/example/demo/issues/12/comments "));
            assert!(request.contains(r#""body":"#));
        },
        r#"{"id":12}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "comment",
            "create",
            "--repo",
            "example/demo",
            "--number",
            "12",
            "--body",
            &too_long,
            "--body-max=0",
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");

    let body_update = "y".repeat(150);
    let (api_base, handle) = mock_github(
        |request| {
            assert!(request.starts_with("PATCH /repos/example/demo/issues/12 "));
            assert!(request.contains(r#""body":"#));
        },
        r#"{"number":12}"#,
    );
    let output = bin()
        .current_dir(&cwd)
        .env("PATH", prepend_path_for_test(&bin_dir))
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .env("RUNSEAL_GITHUB_API_BASE", api_base)
        .env("GITHUB_TOKEN", "env-token")
        .args([
            "@tool",
            "github",
            "issue",
            "body",
            "update",
            "--repo",
            "example/demo",
            "--number",
            "12",
            "--body",
            &body_update,
        ])
        .output()
        .expect("runseal should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    handle.join().expect("mock server should finish");
}

fn write_git_stub(path: &Path, origin: &str, branch: &str) {
    write_executable(
        path,
        &format!(
            r#"#!/usr/bin/env sh
set -eu
if [ "$1" = "remote" ] && [ "${{2:-}}" = "get-url" ] && [ "${{3:-}}" = "origin" ]; then
  printf '%s\n' '{}'
  exit 0
fi
if [ "$1" = "branch" ] && [ "${{2:-}}" = "--show-current" ]; then
  printf '%s\n' '{}'
  exit 0
fi
exit 1
"#,
            origin, branch
        ),
    );
}

fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, content).expect("stub should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("stub metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("stub should be executable");
}

fn prepend_path_for_test(bin_dir: &Path) -> OsString {
    let mut paths = vec![PathBuf::from(bin_dir)];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("PATH should be joinable")
}

fn mock_github<F>(assert_request: F, body: &'static str) -> (String, thread::JoinHandle<()>)
where
    F: FnOnce(&str) + Send + 'static,
{
    let server = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
    let address = server
        .local_addr()
        .expect("mock server address should exist");
    let handle = thread::spawn(move || {
        let mut stream = accept_with_timeout(&server);
        let mut request = [0_u8; 8192];
        let read = stream
            .read(&mut request)
            .expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert_request(&request);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("response should be written");
    });
    (format!("http://{address}"), handle)
}

fn accept_with_timeout(server: &TcpListener) -> std::net::TcpStream {
    server
        .set_nonblocking(true)
        .expect("mock server should become nonblocking");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match server.accept() {
            Ok((stream, _)) => return stream,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => wait_request(deadline),
            Err(err) => panic!("mock accept failed: {err}"),
        }
    }
}

fn wait_request(deadline: Instant) {
    if Instant::now() >= deadline {
        panic!("mock request did not arrive within 5 seconds");
    }
    thread::sleep(Duration::from_millis(10));
}
