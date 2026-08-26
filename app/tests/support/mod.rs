use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

use tempfile::TempDir;

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output should be UTF-8")
}

pub struct Seat {
    _temp: TempDir,
    pub home: std::path::PathBuf,
    cwd: std::path::PathBuf,
}

impl Seat {
    pub fn new() -> Self {
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

    pub fn write(&self, url: &str) {
        let body = format!(
            "[env.vars]\nFORGEJO_URL = \"{url}\"\nFORGEJO_TOKEN_FILE = \"local://secrets/forgejo\"\n"
        );
        std::fs::write(self.home.join("profiles/perish/runseal.toml"), body)
            .expect("profile should be written");
    }

    pub fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_runseal"))
            .current_dir(&self.cwd)
            .env("RUNSEAL_HOME", &self.home)
            .args(args)
            .output()
            .expect("Runseal should execute")
    }
}

pub fn serve(status: &str, body: &str, hits: usize) -> (String, thread::JoinHandle<Vec<String>>) {
    sequence(vec![(status.to_string(), body.to_string()); hits])
}

pub fn sequence(answers: Vec<(String, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in answers {
            let (stream, _) = listener.accept().expect("accept");
            seen.push(reply(stream, &status, &body));
        }
        seen
    });
    (format!("http://{addr}"), handle)
}

fn reply(mut stream: std::net::TcpStream, status: &str, body: &str) -> String {
    let head = drain(&mut stream);
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nRetry-After: 0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes());
    head
}

fn drain(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while let Ok(n) = stream.read(&mut chunk) {
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(at) = buf.windows(4).position(|held| held == b"\r\n\r\n")
            && buf.len() >= at + 4 + length(&buf[..at])
        {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn length(head: &[u8]) -> usize {
    String::from_utf8_lossy(head)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}
