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
    assert!(stdout.contains(&face(home.join("profiles/perish/runseal.toml"))));
    assert!(stdout.contains(&format!(
        "RUNSEAL_ROOT={}",
        home.join("profiles/perish").display()
    )));
    let resolved = invoke(
        &home,
        &cwd,
        &["resolve", "--profile", "perish", "local://secrets/forgejo"],
    );
    assert!(resolved.status.success(), "{}", text(&resolved.stderr));
    assert!(
        text(&resolved.stdout).contains(&face(home.join("profiles/perish/.local/secrets/forgejo")))
    );
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
    assert!(stdout.contains(&face(home.join("profiles/perish.toml"))));
    assert!(!stdout.contains(&face(home.join("profiles/perish/runseal.toml"))));
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
    assert!(stderr.contains(&face(home.join("profiles/perish.toml"))));
    assert!(stderr.contains(&face(home.join("profiles/perish/runseal.toml"))));
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
    assert!(stdout.contains(&face(home.join("profiles/default/runseal.toml"))));
}

fn face(path: impl AsRef<std::path::Path>) -> String {
    path.as_ref().display().to_string()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output should be UTF-8")
}
