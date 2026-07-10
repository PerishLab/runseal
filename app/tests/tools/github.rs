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

struct Fixture {
    _temp: TempDir,
    repo: PathBuf,
    tools: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should be created");
        let repo = temp.path().join("repo");
        let tools = temp.path().join("bin");
        std::fs::create_dir_all(&repo).expect("cwd should be created");
        std::fs::create_dir_all(&tools).expect("bin dir should be created");
        let fixture = Self {
            _temp: temp,
            repo,
            tools,
        };
        fixture.git("git@gitee.com:perishme/perish.top.git", "feat/prefix");
        fixture
    }

    fn git(&self, origin: &str, branch: &str) {
        Script::write(
            &self.tools.join("git"),
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

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.repo.join(name);
        std::fs::write(&path, content).expect("fixture file should be written");
        path
    }

    fn run(&self, args: &[&str], vars: &[(&str, String)]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
        command
            .current_dir(&self.repo)
            .env("PATH", self.path())
            .env("RUNSEAL_HOME", self._temp.path().join("home"))
            .args(args);
        for (key, value) in vars {
            command.env(key, value);
        }
        command.output().expect("runseal should run")
    }

    fn path(&self) -> OsString {
        let mut paths = vec![self.tools.clone()];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }
}

struct Script;

impl Script {
    fn write(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, content).expect("stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("stub should be executable");
    }
}

struct Mock;

impl Mock {
    fn serve<F>(assertion: F, body: &'static str) -> (String, thread::JoinHandle<()>)
    where
        F: FnOnce(&str) + Send + 'static,
    {
        let server = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
        let address = server
            .local_addr()
            .expect("mock server address should exist");
        let handle = thread::spawn(move || {
            let mut stream = Self::accept(&server);
            let mut request = [0_u8; 8192];
            let read = stream
                .read(&mut request)
                .expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..read]);
            assertion(&request);
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

    fn accept(server: &TcpListener) -> std::net::TcpStream {
        server
            .set_nonblocking(true)
            .expect("mock server should become nonblocking");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match server.accept() {
                Ok((stream, _)) => return stream,
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Self::wait(deadline),
                Err(err) => panic!("mock accept failed: {err}"),
            }
        }
    }

    fn wait(deadline: Instant) {
        if Instant::now() >= deadline {
            panic!("mock request did not arrive within 5 seconds");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

mod comment {
    use super::*;

    #[test]
    fn writes() {
        let fx = Fixture::new();
        let body = fx.write("body.md", "Hello from file\n");
        let token = fx.write("github.env", "GITHUB_TOKEN=file-token\n");
        let (api, handle) = Mock::serve(
            |request| {
                assert!(request.starts_with("POST /repos/PerishCode/runseal/issues/46/comments "));
                assert!(request.contains("authorization: Bearer file-token"));
                assert!(request.contains(
                    r#"Requested-By-Repo: perishme/perish.top\nRequested-By-Branch: feat/prefix\n\nHello from file\n"#
                ));
            },
            r#"{"id":46,"html_url":"https://github.test/comment/46"}"#,
        );
        let output = fx.run(
            &[
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
                body.to_str().unwrap(),
                "--prefix-enable=true",
                "--token-file",
                token.to_str().unwrap(),
            ],
            &[("RUNSEAL_GITHUB_API_BASE", api)],
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        handle.join().expect("mock server should finish");

        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
        assert_eq!(payload["id"], 46);

        let (api, handle) = Mock::serve(
            |request| {
                assert!(request.starts_with("PATCH /repos/perishme/perish.top/issues/77 "));
                assert!(request.contains("authorization: Bearer explicit-token"));
                assert!(request.contains(r#""body":"Already prefixed""#));
            },
            r#"{"number":77,"html_url":"https://github.test/issues/77"}"#,
        );
        let output = fx.run(
            &[
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
            ],
            &[
                ("RUNSEAL_GITHUB_API_BASE", api),
                ("GITHUB_TOKEN", "env-token".to_string()),
            ],
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        handle.join().expect("mock server should finish");
    }
}

mod issue {
    use super::*;

    #[test]
    fn creates() {
        let fx = Fixture::new();
        let body = fx.write("body.md", "Issue body\n");
        let (api, handle) = Mock::serve(
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
        let output = fx.run(
            &[
                "@tool",
                "github",
                "issue",
                "create",
                "--repo",
                "PerishCode/runseal",
                "--title",
                "Open one issue",
                "--body-file",
                body.to_str().unwrap(),
                "--prefix-enable=true",
            ],
            &[
                ("RUNSEAL_GITHUB_API_BASE", api),
                ("GITHUB_TOKEN", "env-token".to_string()),
            ],
        );
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
}

mod prefix {
    use super::*;

    #[test]
    fn applies() {
        let fx = Fixture::new();
        fx.git("", "feat/prefix");
        let failed = fx.run(
            &[
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
            ],
            &[("GITHUB_TOKEN", "env-token".to_string())],
        );
        assert!(!failed.status.success());
        assert!(
            String::from_utf8_lossy(&failed.stderr)
                .contains("cannot parse owner/repo from origin url"),
            "stderr: {}",
            String::from_utf8_lossy(&failed.stderr)
        );

        fx.git("git@gitee.com:perishme/perish.top.git", "feat/prefix");
        let (api, handle) = Mock::serve(
            |request| {
                assert!(request.starts_with("POST /repos/PerishCode/runseal/issues/12/comments "));
                assert!(request.contains(r#""body":"Hello""#));
                assert!(!request.contains("Requested-By-Repo:"));
            },
            r#"{"id":12}"#,
        );
        let output = fx.run(
            &[
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
            ],
            &[
                ("RUNSEAL_GITHUB_API_BASE", api),
                ("GITHUB_TOKEN", "env-token".to_string()),
            ],
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        handle.join().expect("mock server should finish");
    }
}

mod body {
    use super::*;

    #[test]
    fn limits() {
        let fx = Fixture::new();
        let text = "x".repeat(101);
        let failed = fx.run(
            &[
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
                &text,
            ],
            &[("GITHUB_TOKEN", "env-token".to_string())],
        );
        assert!(!failed.status.success());
        assert!(
            String::from_utf8_lossy(&failed.stderr)
                .contains("body length 101 exceeds --body-max=100"),
            "stderr: {}",
            String::from_utf8_lossy(&failed.stderr)
        );

        let (api, handle) = Mock::serve(
            |request| {
                assert!(request.starts_with("POST /repos/example/demo/issues/12/comments "));
                assert!(request.contains(r#""body":"#));
            },
            r#"{"id":12}"#,
        );
        let output = fx.run(
            &[
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
                &text,
                "--body-max=0",
            ],
            &[
                ("RUNSEAL_GITHUB_API_BASE", api),
                ("GITHUB_TOKEN", "env-token".to_string()),
            ],
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        handle.join().expect("mock server should finish");

        let update = "y".repeat(150);
        let (api, handle) = Mock::serve(
            |request| {
                assert!(request.starts_with("PATCH /repos/example/demo/issues/12 "));
                assert!(request.contains(r#""body":"#));
            },
            r#"{"number":12}"#,
        );
        let output = fx.run(
            &[
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
                &update,
            ],
            &[
                ("RUNSEAL_GITHUB_API_BASE", api),
                ("GITHUB_TOKEN", "env-token".to_string()),
            ],
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        handle.join().expect("mock server should finish");
    }
}
