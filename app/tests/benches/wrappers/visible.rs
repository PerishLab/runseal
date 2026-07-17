#![allow(unused_imports)]
use super::seat::*;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

#[test]
fn visible() {
    let fx = Fixture::new();
    Wrapper::ts(&File::ts(&fx.local, "wrap"));
    Wrapper::shell(&File::shell(&fx.global, "wrap"), "home");
    Wrapper::ts(&File::ts(&fx.global, "home-only"));

    let output = fx.run(&["@wrappers"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(":wrap"));
    assert!(stdout.contains(":home-only"));
    assert!(stdout.contains("profile"));
    assert!(stdout.contains("home"));
    let line = stdout
        .lines()
        .find(|line| line.contains(":wrap"))
        .expect("wrap should be listed");
    let file = line
        .split_whitespace()
        .last()
        .expect("wrap line should include a file");
    assert!(line.contains("profile"));
    assert!(
        Path::new(file).ends_with(Paths::suffix(&File::ts(&fx.local, "wrap"), 4)),
        "expected {file} to point at the profile wrapper"
    );
}

#[test]
fn resolve() {
    let fx = Fixture::new();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);

    let which = fx.run(&["@which", ":tool"]);

    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &wrapper);
}

#[test]
fn extension() {
    let fx = Fixture::new();
    Wrapper::shell(&fx.local.join("legacy"), "legacy");

    let output = fx.run(&[":legacy"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("wrapper not found: :legacy"));
    assert!(stderr.contains("legacy.ts"));
    assert!(stderr.contains("legacy.sh"));
    assert!(!stderr.contains(".runseal/wrappers/legacy\n"));
}

#[test]
fn policy() {
    let fx = Fixture::new();
    let log = fx.deno();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", Paths::prepend(&fx.bin))
        .env("RUNSEAL_TEST_DENO_LOG", &log)
        .args([":tool", "hello", "world"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("name=tool"));
    Assert::path(
        stdout
            .lines()
            .find_map(|line| line.strip_prefix("file="))
            .expect("stdout should include wrapper file"),
        &wrapper,
    );
    let log = std::fs::read_to_string(log).expect("deno log should be readable");
    assert!(log.contains("deno run --no-prompt"));
    assert!(log.contains("--config"));
    assert!(log.contains(".runseal/deno.json"));
    assert!(log.contains("--lock"));
    assert!(log.contains(".runseal/deno.lock"));
    assert!(log.contains("--frozen=true"));
    assert!(log.contains("--allow-env"));
    assert!(log.contains("hello world"));
}
