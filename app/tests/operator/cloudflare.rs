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

struct Mock;

impl Mock {
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
