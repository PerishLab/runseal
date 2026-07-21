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
    project: PathBuf,
    bin: PathBuf,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let bin = temp.path().join("bin");
    std::fs::create_dir_all(project.join(".runseal/wrappers"))
        .expect("wrapper dir should be created");
    std::fs::create_dir_all(project.join("app/tests")).expect("app tests dir should be created");
    std::fs::create_dir_all(&bin).expect("bin dir should be created");

    std::fs::write(
        project.join("runseal.toml"),
        r#"
injections = []

[deno]
config = ".runseal/deno.json"
lock = ".runseal/deno.lock"
permissions = [
  "--allow-read=.",
  "--allow-env",
  "--allow-net=127.0.0.1",
  "--allow-run=git,cargo,negentropy,runseal",
]
"#,
    )
    .expect("profile should be written");
    std::fs::write(
        project.join(".runseal/deno.json"),
        std::fs::read_to_string(Fixture::root().join(".runseal/deno.json"))
            .expect("repo deno config should be readable"),
    )
    .expect("deno config should be copied");
    std::fs::write(
        project.join(".runseal/deno.lock"),
        std::fs::read_to_string(Fixture::root().join(".runseal/deno.lock"))
            .expect("repo deno lock should be readable"),
    )
    .expect("deno lock should be copied");
    std::fs::write(
        project.join(".runseal/wrappers/guard.ts"),
        std::fs::read_to_string(Fixture::root().join(".runseal/wrappers/guard.ts"))
            .expect("repo guard wrapper should be readable"),
    )
    .expect("guard wrapper should be copied");
    std::fs::write(project.join("app/tests/sample.txt"), "sample\n")
        .expect("sample test file should be written");

    Script::write(
        &bin.join("git"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${1:-}" = "rev-parse" ] && [ "${2:-}" = "--show-toplevel" ]; then
  printf '%s\n' "${RUNSEAL_TEST_ROOT:?}"
  exit 0
fi
exit 0
"#,
    );
    Script::write(
        &bin.join("cargo"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${1:-}" = "metadata" ]; then
  if [ -n "${RUNSEAL_TEST_CARGO_METADATA:-}" ]; then
    printf '%s\n' "$RUNSEAL_TEST_CARGO_METADATA"
  else
    printf '%s\n' '{"packages":[{"version":"0.6.1"}]}'
  fi
  exit 0
fi
exit 0
"#,
    );
    Fixture {
        _temp: temp,
        project,
        bin,
    }
}

impl Fixture {
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app dir should have repo parent")
            .to_path_buf()
    }

    fn path(&self) -> OsString {
        let mut paths = vec![self.bin.clone()];
        if let Some(runseal) = Path::new(env!("CARGO_BIN_EXE_runseal")).parent() {
            paths.push(runseal.to_path_buf());
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }

    fn run(&self, args: &[&str], vars: &[(&str, &str)]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
        command
            .current_dir(&self.project)
            .env("PATH", self.path())
            .env("RUNSEAL_TEST_ROOT", &self.project)
            .arg("-p")
            .arg(self.project.join("runseal.toml"))
            .arg(":guard")
            .args(args);
        for (key, value) in vars {
            command.env(key, value);
        }
        command.output().expect("guard should run")
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
    fn serve(status: u16, body: &'static str) -> (String, thread::JoinHandle<()>) {
        let server = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
        let address = server
            .local_addr()
            .expect("mock server address should exist");
        let handle = thread::spawn(move || {
            let mut stream = Self::accept(&server);
            let mut request = [0_u8; 2048];
            let read = stream
                .read(&mut request)
                .expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /metadata.json?version="));
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("response should be written");
        });
        (format!("http://{address}/metadata.json"), handle)
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

mod version {
    use super::*;

    #[test]
    fn hashes() {
        let fx = fixture();

        let wrapper = fx.run(&["version-hash"], &[]);
        assert!(
            wrapper.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&wrapper.stderr)
        );
        let stdout = String::from_utf8(wrapper.stdout).expect("stdout should be UTF-8");
        assert_eq!(stdout.trim().len(), 64);
        assert!(stdout.trim().chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}

mod stable {
    use super::*;

    #[test]
    fn skips() {
        let fx = fixture();
        let (url, handle) = Mock::serve(404, "");

        let output = fx.run(
            &["version-check"],
            &[("RUNSEAL_STABLE_METADATA_URL", url.as_str())],
        );

        handle.join().expect("mock server should finish");
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("guard version policy: no stable metadata; skipping")
        );
    }
}

mod bump {
    use super::*;

    #[test]
    fn rejects() {
        let fx = fixture();
        let (url, handle) = Mock::serve(
            200,
            r#"{"stableVersion":"0.6.0","guard":{"version":{"hash":"different"}}}"#,
        );

        let output = fx.run(
            &["version-check"],
            &[
                (
                    "RUNSEAL_TEST_CARGO_METADATA",
                    r#"{"packages":[{"version":"0.6.1"}]}"#,
                ),
                ("RUNSEAL_STABLE_METADATA_URL", url.as_str()),
            ],
        );

        handle.join().expect("mock server should finish");
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("changed guard.version.hash requires a minor-or-higher bump")
        );
    }
}
