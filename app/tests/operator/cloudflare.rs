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
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    std::fs::create_dir_all(project.join(".runseal/wrappers"))
        .expect("wrapper dir should be created");
    std::fs::create_dir_all(project.join(".runseal/templates"))
        .expect("template dir should be created");
    std::fs::write(
        project.join("runseal.toml"),
        r#"
[resources]
root = ".local"

[deno]
config = ".runseal/deno.json"
lock = ".runseal/deno.lock"
permissions = [
  "--allow-read",
  "--allow-write",
  "--allow-env",
  "--allow-net",
  "--allow-run=runseal",
]

[[injections]]
type = "env"

[injections.vars]
RUNSEAL_REPO_LOCAL_DIR = "resource://"
RUNSEAL_REPO_SECRETS_DIR = "resource://secrets"
RUNSEAL_REPO_TMP_DIR = "resource://tmp"
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
        project.join(".runseal/wrappers/cloudflare.ts"),
        std::fs::read_to_string(Fixture::root().join(".runseal/wrappers/cloudflare.ts"))
            .expect("repo cloudflare wrapper should be readable"),
    )
    .expect("cloudflare wrapper should be copied");
    std::fs::write(
        project.join(".runseal/templates/cloudflare.env"),
        std::fs::read_to_string(Fixture::root().join(".runseal/templates/cloudflare.env"))
            .expect("repo cloudflare template should be readable"),
    )
    .expect("cloudflare template should be copied");
    Fixture {
        _temp: temp,
        project,
    }
}

impl Fixture {
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app dir should have repo parent")
            .to_path_buf()
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        self.env(args, &[])
    }

    fn env(&self, args: &[&str], envs: &[(&str, String)]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
        command
            .current_dir(&self.project)
            .env("PATH", Self::path())
            .arg("-p")
            .arg(self.project.join("runseal.toml"))
            .arg(":cloudflare")
            .args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        command.output().expect("cloudflare wrapper should run")
    }

    fn path() -> OsString {
        let mut paths = Vec::new();
        if let Some(runseal) = Path::new(env!("CARGO_BIN_EXE_runseal")).parent() {
            paths.push(runseal.to_path_buf());
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }

    fn write(&self) {
        let secrets = self.project.join(".local/secrets");
        std::fs::create_dir_all(&secrets).expect("secrets dir should be created");
        std::fs::write(
            secrets.join("cloudflare.env"),
            "\
CLOUDFLARE_ACCOUNT_ID=account-123
CLOUDFLARE_API_TOKEN=token-456
CLOUDFLARE_ZONE_NAME=perish.uk
CLOUDFLARE_MANAGE_HOST=runseal.perish.uk
CLOUDFLARE_MANAGE_ORIGIN_HOST=releases.runseal.perish.uk
CLOUDFLARE_MANAGE_REDIRECT_PREFIX=
",
        )
        .expect("credentials should be written");
    }
}

struct Tool;

impl Tool {
    fn run(args: &[&str], envs: &[(&str, String)]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
        command.args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        command.output().expect("cloudflare tool should run")
    }

    fn credentials() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("temp dir should be created");
        let secrets = temp.path().join("secrets");
        std::fs::create_dir_all(&secrets).expect("secrets dir should be created");
        std::fs::write(
            secrets.join("cloudflare.env"),
            "\
CLOUDFLARE_ACCOUNT_ID=account-123
CLOUDFLARE_API_TOKEN=token-456
",
        )
        .expect("credentials should be written");
        (temp, secrets)
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
            let mut request = [0_u8; 4096];
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

    fn assert(request: &str, expected: &str) {
        let body = request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("request should include a body");
        let actual: serde_json::Value = serde_json::from_str(body).expect("body should be JSON");
        let expected: serde_json::Value =
            serde_json::from_str(expected).expect("expected body should be JSON");
        assert_eq!(actual, expected);
    }
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

mod init {
    use super::*;

    #[test]
    fn writes() {
        let fx = fixture();

        let output = fx.run(&["init"]);

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(stdout(&output).contains("created"));
        let token = fx.project.join(".local/secrets/cloudflare.env");
        let text = std::fs::read_to_string(token).expect("token template should exist");
        assert!(text.contains("CLOUDFLARE_ACCOUNT_ID="));
        assert!(text.contains("CLOUDFLARE_ZONE_NAME=perish.uk"));
    }
}

mod manage {
    use super::*;

    #[test]
    fn plans() {
        let fx = fixture();
        fx.write();

        let output = fx.run(&["manage-plan"]);

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let stdout = stdout(&output);
        assert!(stdout.contains("manage redirect plan"));
        assert!(stdout.contains("runseal_manage_sh_redirect"));
        assert!(stdout.contains("https://releases.runseal.perish.uk/manage.sh"));
        assert!(!stdout.contains("manage.ps1"));
    }
}

mod zone {
    use super::*;

    #[test]
    fn gets() {
        let (_temp, secrets) = Tool::credentials();
        let (api_base, handle) = Mock::serve(
            move |request| {
                assert!(request.starts_with("GET /zones?name=perish.uk "));
                assert!(request.contains("authorization: Bearer token-456"));
            },
            r#"{"success":true,"result":[{"id":"zone-123","name":"perish.uk","status":"active"}]}"#,
        );

        let output = Tool::run(
            &["@tool", "cloudflare", "zone", "get", "--name", "perish.uk"],
            &[
                (
                    "RUNSEAL_REPO_SECRETS_DIR",
                    secrets.to_string_lossy().into_owned(),
                ),
                ("CLOUDFLARE_API_BASE", api_base),
            ],
        );

        handle.join().expect("mock server should finish");
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert_eq!(
            stdout(&output),
            r#"{"id":"zone-123","name":"perish.uk","status":"active"}"#.to_string() + "\n"
        );
    }
}

mod dns {
    use super::*;

    #[test]
    fn lists() {
        let (_temp, secrets) = Tool::credentials();
        let (api_base, handle) = Mock::serve(
            move |request| {
                assert!(
                    request.starts_with("GET /zones/zone-123/dns_records?name=sidecar.perish.uk ")
                );
                assert!(request.contains("authorization: Bearer token-456"));
            },
            r#"{"success":true,"result":[{"id":"record-123","name":"sidecar.perish.uk"}]}"#,
        );

        let output = Tool::run(
            &[
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "list",
                "--zone-id",
                "zone-123",
                "--name",
                "sidecar.perish.uk",
            ],
            &[
                (
                    "RUNSEAL_REPO_SECRETS_DIR",
                    secrets.to_string_lossy().into_owned(),
                ),
                ("CLOUDFLARE_API_BASE", api_base),
            ],
        );

        handle.join().expect("mock server should finish");
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert_eq!(
            stdout(&output),
            r#"[{"id":"record-123","name":"sidecar.perish.uk"}]"#.to_string() + "\n"
        );
    }

    #[test]
    fn creates() {
        let (_temp, secrets) = Tool::credentials();
        let record = r#"{"type":"CNAME","name":"sidecar.perish.uk","content":"releases.sidecar.perish.uk","ttl":1,"proxied":true}"#;
        let (api_base, handle) = Mock::serve(
            move |request| {
                assert!(request.starts_with("POST /zones/zone-123/dns_records "));
                assert!(request.contains("authorization: Bearer token-456"));
                Mock::assert(request, record);
            },
            r#"{"success":true,"result":{"id":"record-123","type":"CNAME"}}"#,
        );

        let output = Tool::run(
            &[
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "create",
                "--zone-id",
                "zone-123",
                "--json",
                record,
            ],
            &[
                (
                    "RUNSEAL_REPO_SECRETS_DIR",
                    secrets.to_string_lossy().into_owned(),
                ),
                ("CLOUDFLARE_API_BASE", api_base),
            ],
        );

        handle.join().expect("mock server should finish");
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert_eq!(
            stdout(&output),
            r#"{"id":"record-123","type":"CNAME"}"#.to_string() + "\n"
        );
    }

    #[test]
    fn updates() {
        let (_temp, secrets) = Tool::credentials();
        let record = r#"{"type":"CNAME","name":"sidecar.perish.uk","content":"releases.sidecar.perish.uk","ttl":1,"proxied":true}"#;
        let (api_base, handle) = Mock::serve(
            move |request| {
                assert!(request.starts_with("PATCH /zones/zone-123/dns_records/record-123 "));
                assert!(request.contains("authorization: Bearer token-456"));
                Mock::assert(request, record);
            },
            r#"{"success":true,"result":{"id":"record-123","modified":true}}"#,
        );

        let output = Tool::run(
            &[
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "update",
                "--zone-id",
                "zone-123",
                "--record-id",
                "record-123",
                "--json",
                record,
            ],
            &[
                (
                    "RUNSEAL_REPO_SECRETS_DIR",
                    secrets.to_string_lossy().into_owned(),
                ),
                ("CLOUDFLARE_API_BASE", api_base),
            ],
        );

        handle.join().expect("mock server should finish");
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert_eq!(
            stdout(&output),
            r#"{"id":"record-123","modified":true}"#.to_string() + "\n"
        );
    }

    #[test]
    fn rejects() {
        let (_temp, secrets) = Tool::credentials();

        let output = Tool::run(
            &[
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "create",
                "--zone-id",
                "zone-123",
                "--json",
                "{",
            ],
            &[(
                "RUNSEAL_REPO_SECRETS_DIR",
                secrets.to_string_lossy().into_owned(),
            )],
        );

        assert!(!output.status.success());
        assert!(stderr(&output).contains("invalid DNS record JSON"));
    }
}

mod api {
    use super::*;

    #[test]
    fn passes() {
        let fx = fixture();
        fx.write();
        let server = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
        let address = server
            .local_addr()
            .expect("mock server address should exist");
        let handle = thread::spawn(move || {
            let mut stream = Mock::accept(&server);
            let mut request = [0_u8; 2048];
            let read = stream
                .read(&mut request)
                .expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /zones?name=perish.uk "));
            let body = r#"{"success":true,"result":[{"id":"zone-123"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("response should be written");
        });

        let output = fx.env(
            &["api", "GET", "/zones", "--query", "name=perish.uk"],
            &[("CLOUDFLARE_API_BASE", format!("http://{address}"))],
        );

        handle.join().expect("mock server should finish");
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(stdout(&output).contains(r#""id":"zone-123""#));
    }
}
