use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output should be UTF-8")
}

struct Seat {
    _temp: TempDir,
    home: std::path::PathBuf,
    cwd: std::path::PathBuf,
}

impl Seat {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should exist");
        let home = temp.path().join("held");
        let cwd = temp.path().join("work");
        std::fs::create_dir_all(&cwd).expect("work should exist");
        std::fs::create_dir_all(home.join("profiles/perish/.local/secrets"))
            .expect("secrets should exist");
        std::fs::write(
            home.join("profiles/perish/.local/secrets/forgejo"),
            "secret-token\n",
        )
        .expect("token should exist");
        Self {
            _temp: temp,
            home,
            cwd,
        }
    }

    fn write(&self, url: &str) {
        let body = format!(
            "[env.vars]\nFORGEJO_URL = \"{url}\"\nFORGEJO_TOKEN_FILE = \"local://secrets/forgejo\"\n"
        );
        std::fs::write(self.home.join("profiles/perish/runseal.toml"), body)
            .expect("profile should be written");
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.cwd)
            .env("RUNSEAL_HOME", &self.home)
            .args(args)
            .output()
            .expect("Runseal should execute")
    }
}

fn serve(status: &str, body: &str, hits: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let status = status.to_string();
    let body = body.to_string();
    let handle = thread::spawn(move || {
        for _ in 0..hits {
            let (stream, _) = listener.accept().expect("accept");
            reply(stream, &status, &body);
        }
    });
    (format!("http://{addr}"), handle)
}

fn reply(mut stream: std::net::TcpStream, status: &str, body: &str) {
    drain(&mut stream);
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes());
}

fn drain(stream: &mut std::net::TcpStream) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|held| held == b"\r\n\r\n") {
        let Ok(n) = stream.read(&mut chunk) else {
            break;
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

#[test]
fn absent() {
    let seat = Seat::new();
    std::fs::write(seat.home.join("profiles/perish/runseal.toml"), "").expect("empty");
    let output = seat.run(&[":perish", "@forgejo", "user", "show"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("FORGEJO_URL"));
}

#[test]
fn user() {
    let (url, handle) = serve("200 OK", r#"{"login":"perish"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "--json", "user", "show"]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains(r#""version":1"#) || stdout.contains(r#""version": 1"#));
    assert!(stdout.contains(r#""login":"perish""#) || stdout.contains(r#""login": "perish""#));
}

#[test]
fn issue() {
    let (url, handle) = serve("200 OK", r#"{"number":154,"state":"open","title":"K1"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "issue", "show", "PerishLab/keel#154"]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "154\topen\tK1");
}

#[test]
fn listed() {
    let (url, handle) = serve(
        "200 OK",
        r#"[{"number":1,"state":"open","title":"one"}]"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/keel",
        "issue",
        "list",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "1\topen\tone");
}

#[test]
fn refused() {
    let (url, handle) = serve("401 Unauthorized", r#"{"message":"unauthorized"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "user", "show"]);
    handle.join().expect("server");
    assert!(!output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stderr).contains("unauthorized"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn unknown() {
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[":", "@missing"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unknown Runseal tool: @missing"));
}
