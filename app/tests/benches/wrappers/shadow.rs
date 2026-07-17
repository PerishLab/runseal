#![allow(unused_imports)]
use super::seat::*;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

#[test]
fn expand() {
    let fx = Fixture::new();
    let log = fx.deno();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);
    std::fs::write(
        fx.project.join("runseal.toml"),
        r#"
injections = []

[resources]
root = ".resource"

[deno]
permissions = ["--allow-read=${RUNSEAL_TEST_READ}"]
"#,
    )
    .expect("profile should be written");
    let allowed = fx.home.join("tea.yml");

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("RUNSEAL_TEST_READ", &allowed)
        .env("RUNSEAL_TEST_DENO_LOG", &log)
        .env("PATH", Paths::prepend(&fx.bin))
        .args([":tool"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let log = std::fs::read_to_string(log).expect("deno log should be readable");
    assert!(log.contains(&format!("--allow-read={}", allowed.display())));
    assert!(!log.contains("${RUNSEAL_TEST_READ}"));
}

#[test]
fn required() {
    let fx = Fixture::new();
    std::fs::write(
        fx.project.join("runseal.toml"),
        "injections = []\n[resources]\nroot = \".resource\"\n",
    )
    .expect("profile should be written");
    Wrapper::ts(&File::ts(&fx.local, "tool"));

    let output = fx.run(&[":tool"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("deno wrapper requires a [deno] profile policy"));
}

#[test]
fn precedence() {
    let fx = Fixture::new();
    let log = fx.deno();
    Wrapper::ts(&File::ts(&fx.local, "tool"));
    Wrapper::shell(&File::shell(&fx.local, "tool"), "shell");

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", Paths::prepend(&fx.bin))
        .env("RUNSEAL_TEST_DENO_LOG", &log)
        .args([":tool"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("name=tool"));
    assert!(!stdout.contains("shell"));
}

#[test]
fn shell() {
    let fx = Fixture::new();
    Wrapper::shell(&File::shell(&fx.local, "tool"), "shell");

    let output = fx.run(&[":tool"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "shell\n");
}

#[test]
fn shadow() {
    let fx = Fixture::new();
    let wrapper = File::ts(&fx.local, "wrap");
    Wrapper::ts(&wrapper);
    Wrapper::shell(&File::shell(&fx.global, "wrap"), "home");

    let which = fx.run(&["@which", ":wrap"]);
    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &wrapper);

    let output = fx.run(&["@wrappers"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let lines = stdout
        .lines()
        .filter(|line| line.starts_with(":wrap "))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("profile"));
}
