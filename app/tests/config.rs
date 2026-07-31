use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[test]
fn home() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let output = bin()
        .current_dir(temp.path())
        .env("RUNSEAL_HOME", &home)
        .args(["profile"])
        .output()
        .expect("Runseal should execute");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(&format!("RUNSEAL_HOME={}", home.display())));
    assert!(stdout.contains("RUNSEAL_PROFILE=default"));
    assert!(!stdout.contains("RUNSEAL_PROFILE_PATH="));
}

#[test]
fn file() {
    let temp = TempDir::new().expect("temp dir should exist");
    std::fs::write(temp.path().join("runseal.toml"), "[env").expect("profile should be written");
    let output = bin()
        .current_dir(temp.path())
        .args(["profile"])
        .output()
        .expect("Runseal should execute");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("cannot parse"));
    assert!(stderr.contains("runseal.toml"));
}

#[test]
fn schema() {
    let temp = TempDir::new().expect("temp dir should exist");
    std::fs::write(
        temp.path().join("runseal.toml"),
        "[deno]\nlock = \"deno.lock\"\n",
    )
    .expect("profile should be written");
    let output = bin()
        .current_dir(temp.path())
        .args(["profile"])
        .output()
        .expect("Runseal should execute");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("unknown field"));
    assert!(stderr.contains("deno"));
}

#[test]
fn nested() {
    let temp = TempDir::new().expect("temp dir should exist");
    std::fs::write(
        temp.path().join("runseal.toml"),
        "[env]\nunknown = \"value\"\n",
    )
    .expect("profile should be written");
    let output = bin()
        .current_dir(temp.path())
        .args(["profile"])
        .output()
        .expect("Runseal should execute");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("unknown field"));
    assert!(stderr.contains("unknown"));
}
