#![allow(unused_imports)]
use super::seat::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn help() {
    let output = bin().output().expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--profile"));
}

#[test]
fn explicit() {
    let temp = TempDir::new().expect("temp dir should be created");
    let profile = temp.path().join("profile.toml");
    std::fs::write(
        &profile,
        r#"
[[injections]]
type = "env"

[[injections.ops]]
op = "set_if_absent"
key = "RUNSEAL_TEST_VALUE"
value = "from-toml"
"#,
    )
    .expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(Shell::args(&Script::profile()))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "from-toml|profile.toml");
}

#[test]
fn nearest() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work");
    let home = temp.path().join("home");
    let profiles = home.join("profiles");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&profiles).expect("profile home should be created");
    std::fs::write(
        cwd.join("runseal.yaml"),
        "injections:\n  - type: env\n    vars:\n      PICKED: cwd\n",
    )
    .expect("cwd profile should be written");
    std::fs::write(
        profiles.join("default.toml"),
        "[[injections]]\ntype = \"env\"\n[injections.vars]\nPICKED = \"home\"\n",
    )
    .expect("default profile should be written");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", &home)
        .args(Shell::args(&Script::env("PICKED")))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "cwd");
}

#[cfg(unix)]
#[test]
fn lifecycle() {
    let temp = TempDir::new().expect("temp dir should be created");
    let source = temp.path().join("source.txt");
    let target = temp.path().join("links/source.txt");
    let profile = temp.path().join("profile.json");
    std::fs::write(&source, "sealed").expect("source should be written");
    std::fs::create_dir_all(target.parent().expect("target should have a parent"))
        .expect("target parent should be created");
    std::fs::write(&target, "stale").expect("existing target should be written");
    std::fs::write(
        &profile,
        format!(
            r#"{{
  "injections": [
    {{
      "type": "symlink",
      "source": "{}",
      "target": "{}",
      "on_exist": "replace",
      "cleanup": true
    }}
  ]
}}"#,
            source.display(),
            target.display()
        ),
    )
    .expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(Shell::args(&Script::symlink(&target)))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    assert!(!target.exists(), "symlink should be cleaned after command");
}

#[cfg(unix)]
#[test]
fn contention() {
    let temp = TempDir::new().expect("temp dir should be created");
    let source = temp.path().join("source.txt");
    let target = temp.path().join("links/source.txt");
    let profile = temp.path().join("profile.json");
    std::fs::write(&source, "sealed").expect("source should be written");
    std::fs::write(
        &profile,
        format!(
            r#"{{
  "injections": [
    {{
      "type": "symlink",
      "source": "{}",
      "target": "{}",
      "cleanup": true
    }}
  ]
}}"#,
            source.display(),
            target.display()
        ),
    )
    .expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(Shell::args(&format!("rm -- '{}'", target.display())))
        .output()
        .expect("runseal should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("symlink shutdown failed"));
    assert!(stderr.contains("lifecycle symlink targets are single-owner"));
    assert!(stderr.contains("another concurrent runseal process"));
}
