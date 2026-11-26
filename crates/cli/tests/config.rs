use std::path::{Path, PathBuf};
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

fn invoke(home: &std::path::Path, cwd: &std::path::Path, args: &[&str]) -> std::process::Output {
    bin()
        .current_dir(cwd)
        .env("RUNSEAL_HOME", home)
        .args(args)
        .output()
        .expect("Runseal should execute")
}

fn store(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent should exist");
    }
    std::fs::write(path, body).expect("profile should be written");
}

#[test]
fn seat() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("work should exist");
    store(
        &home.join("profiles/perish/runseal.toml"),
        "[env.vars]\nPICKED = \"nested\"\n",
    );
    let output = invoke(&home, &cwd, &["profile", "perish"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("RUNSEAL_PROFILE=perish"));
    assert!(ends(
        &read(&stdout, "RUNSEAL_PROFILE_PATH"),
        "profiles/perish/runseal.toml",
    ));
    assert!(ends(&read(&stdout, "RUNSEAL_ROOT"), "profiles/perish"));
    let resolved = invoke(
        &home,
        &cwd,
        &["resolve", "--profile", "perish", "local://secrets/token"],
    );
    assert!(resolved.status.success(), "{}", text(&resolved.stderr));
    assert!(ends(
        Path::new(text(&resolved.stdout).trim()),
        "profiles/perish/.local/secrets/token",
    ));
}

#[test]
fn prefer() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("work should exist");
    store(
        &home.join("profiles/perish.toml"),
        "[env.vars]\nPICKED = \"flat\"\n",
    );
    store(
        &home.join("profiles/perish/runseal.toml"),
        "[env.vars]\nPICKED = \"nested\"\n",
    );
    let output = invoke(&home, &cwd, &["profile", "perish"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(ends(
        &read(&stdout, "RUNSEAL_PROFILE_PATH"),
        "profiles/perish.toml",
    ));
}

#[test]
fn vacant() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("work should exist");
    let output = invoke(&home, &cwd, &["profile", "perish"]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("named profile not found: :perish"));
    assert!(stderr.contains("perish.toml"));
    assert!(stderr.contains("runseal.toml"));
}

#[test]
fn usual() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("work should exist");
    store(
        &home.join("profiles/default/runseal.toml"),
        "[env.vars]\nPICKED = \"usual\"\n",
    );
    let output = invoke(&home, &cwd, &["profile"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(ends(
        &read(&stdout, "RUNSEAL_PROFILE_PATH"),
        "profiles/default/runseal.toml",
    ));
}

#[test]
fn manifest() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let root = temp.path().join("repo");
    let nested = root.join("crates/app");
    std::fs::create_dir_all(&nested).expect("nested should exist");
    store(&root.join("plumb.toml"), "[identity]\nns = \"demo\"\n");
    let output = invoke(&home, &nested, &["profile"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(ends(
        &read(&stdout, "RUNSEAL_PROFILE_PATH"),
        "repo/plumb.toml"
    ));
    assert!(ends(&read(&stdout, "RUNSEAL_ROOT"), "repo"));
    assert!(stdout.contains("RUNSEAL_ENV=3"));
    let resolved = invoke(&home, &nested, &["resolve", "local://secrets/token"]);
    assert!(resolved.status.success(), "{}", text(&resolved.stderr));
    assert!(ends(
        Path::new(text(&resolved.stdout).trim()),
        "repo/.local/secrets/token",
    ));
}

#[test]
fn conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("repo should exist");
    store(&root.join("plumb.toml"), "[runseal.env.vars]\nA = \"1\"\n");
    store(&root.join("runseal.toml"), "[env.vars]\nB = \"2\"\n");
    let output = invoke(&home, &root, &["profile"]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("plumb.toml"), "{stderr}");
    assert!(stderr.contains("runseal.toml"), "{stderr}");
    assert!(!stderr.contains("runseal locus:"), "{stderr}");
}

#[test]
fn named() {
    let temp = TempDir::new().expect("temp dir should exist");
    let home = temp.path().join("held");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("work should exist");
    store(
        &home.join("profiles/plumb.toml"),
        "[env.vars]\nPICKED = \"named\"\n",
    );
    let output = invoke(&home, &cwd, &["profile", "plumb"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(ends(
        &read(&stdout, "RUNSEAL_PROFILE_PATH"),
        "profiles/plumb.toml",
    ));
    assert!(stdout.contains("RUNSEAL_ENV=1"));
}

fn read(text: &str, key: &str) -> PathBuf {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing {key} in {text}"))
}

fn ends(path: &Path, tail: &str) -> bool {
    path.ends_with(Path::new(tail))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output should be UTF-8")
}
