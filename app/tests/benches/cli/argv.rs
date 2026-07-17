#![allow(unused_imports)]
use super::seat::*;
use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn argv() {
    let temp = TempDir::new().expect("temp dir should be created");
    let path = temp.path().join("bin");
    let profile = temp.path().join("profile.toml");
    std::fs::create_dir_all(&path).expect("bin dir should be created");
    Probe::write(&path.join("probe"));
    std::fs::write(
        &profile,
        format!(
            r#"
[[injections]]
type = "env"

[[injections.ops]]
op = "prepend"
key = "PATH"
value = "{}"
separator = "os"
dedup = true

[[injections]]
type = "argv"
command = "probe"
args = ["-F", ".local/ssh/config"]
"#,
            path.display()
        ),
    )
    .expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(["probe", "20m.us.zxi"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "-F|.local/ssh/config|20m.us.zxi|");
}

#[test]
fn root() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let cwd = project.join("nested");
    let local = project.join(".runseal/wrappers");
    let global = temp.path().join("home/wrappers");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&local).expect("project wrappers should be created");
    std::fs::create_dir_all(&global).expect("home wrappers should be created");
    std::fs::write(project.join("runseal.toml"), "injections = []\n")
        .expect("profile should be written");
    Wrapper::write(&Wrapper::file(&local, "wrap"), "project");
    Wrapper::write(&Wrapper::file(&global, "wrap"), "home");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg("../runseal.toml")
        .args([":wrap", "arg"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(
        stdout,
        format!("project|arg|wrap|{}|", Wrapper::name("wrap"))
    );
}

#[test]
fn missing() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    std::fs::create_dir_all(&project).expect("project should be created");
    std::fs::write(project.join("runseal.toml"), "injections = []\n")
        .expect("profile should be written");

    let output = bin()
        .current_dir(&project)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .args([":missing"])
        .output()
        .expect("runseal should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("wrapper not found: :missing"));
    assert!(stderr.contains(".runseal"));
    assert!(stderr.contains("wrappers"));
}

#[test]
fn exit() {
    let temp = TempDir::new().expect("temp dir should be created");
    let profile = temp.path().join("profile.json");
    std::fs::write(&profile, r#"{"injections":[]}"#).expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(Shell::args("exit 17"))
        .output()
        .expect("runseal should run");

    assert_eq!(output.status.code(), Some(17));
}

#[test]
fn priority() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::write(
        cwd.join("runseal.toml"),
        "[[injections]]\ntype = \"env\"\n[injections.vars]\nPICKED = \"toml\"\n",
    )
    .expect("toml profile should be written");
    std::fs::write(
        cwd.join("runseal.yaml"),
        "injections:\n  - type: env\n    vars:\n      PICKED: yaml\n",
    )
    .expect("yaml profile should be written");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .args(Shell::args(&Script::env("PICKED")))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "toml");
}

#[test]
fn paths() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work");
    let home = temp.path().join("home");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", &home)
        .args(Shell::args("true"))
        .output()
        .expect("runseal should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("runseal.toml"));
    assert!(stderr.contains("default.json"));
}
