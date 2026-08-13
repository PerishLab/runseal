use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Seat(PathBuf);

impl Seat {
    fn new() -> Self {
        let name = format!(
            "runseal-skill-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path).expect("create temp root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temp root");
    }
}

fn run(seat: &Seat, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
        .args(args)
        .env("RUNSEAL_HOME", seat.path().join("data"))
        .env("HOME", seat.path().join("home"))
        .output()
        .expect("run runseal")
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("utf8")
}

#[test]
fn list() {
    let seat = Seat::new();
    let output = run(&seat, &["skill", "list"]);
    assert!(output.status.success(), "{output:?}");
    assert!(text(&output.stdout).contains("no managed skill"));
}

#[test]
fn help() {
    let seat = Seat::new();
    let output = run(&seat, &["skill", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = text(&output.stdout);
    for deed in ["install", "upgrade", "status", "stage", "list", "uninstall"] {
        assert!(stdout.contains(deed), "{stdout}");
    }
}
