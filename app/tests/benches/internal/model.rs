#![allow(unused_imports)]
use super::seat::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

#[test]
fn model() {
    let output = bin().arg("--help").output().expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("runseal <cmd>"));
    assert!(stdout.contains("runseal :<name>"));
    assert!(stdout.contains("runseal @<name>"));
    assert!(stdout.contains(".ts files are run with deno"));
    assert!(stdout.contains("structured operations over the harness library"));
    assert!(stdout.contains("@profile"));
    assert!(stdout.contains("@resolve"));
    assert!(stdout.contains("current directory upward"));
    assert!(stdout.contains("https://git.perish.top/PerishFire/runseal"));
}

#[test]
fn topics() {
    let fx = Fixture::new();
    for (args, expected) in [
        (vec!["@profile", "--help"], "Usage: runseal @profile"),
        (vec!["@profile", "-h"], "RUNSEAL_PROFILE_PATH"),
        (vec!["@profile", "help"], "Profile discovery"),
        (vec!["@resources", "--help"], "Usage: runseal @resources"),
        (vec!["@resolve", "--help"], "Usage: runseal @resolve"),
        (vec!["@wrappers", "--help"], "Lookup order"),
        (
            vec!["@wrappers", "-h"],
            ".ts wrappers for structured cross-platform operations",
        ),
        (vec!["@which", "--help"], "Usage: runseal @which :<wrapper>"),
    ] {
        let output = fx.run(&args);
        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert!(
            stdout.contains(expected),
            "expected stdout for {args:?} to contain {expected:?}, got {stdout:?}"
        );
    }
}

#[test]
fn unknown() {
    let fx = Fixture::new();

    Assert::fails(
        &fx,
        &["@unknown", "--help"],
        "unknown internal command: @unknown",
    );
}

#[test]
fn profile() {
    let fx = Fixture::new();
    std::fs::write(&fx.profile, "not valid profile toml").expect("profile should be written");

    let output = fx.run(&["@profile"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("RUNSEAL_HOME="));
    assert!(stdout.contains("RUNSEAL_PROFILE_HOME="));
    assert!(stdout.contains("RUNSEAL_PROFILE_PATH="));
    assert!(stdout.contains("RUNSEAL_WRAPPER_PATH="));
    assert!(stdout.contains(fx.profile.to_str().expect("path should be UTF-8")));
}

#[test]
fn resolves() {
    let fx = Fixture::new();
    let wrapper = Wrapper::file(&fx.wrappers, "wrap");
    Wrapper::write(&wrapper, "project");

    let output = fx.run(&["@which", ":wrap"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &wrapper);
}

#[cfg(windows)]
#[test]
fn priority() {
    let fx = Fixture::new();
    let exact = fx.wrappers.join("tool");
    let exe = fx.wrappers.join("tool.exe");
    let cmd = fx.wrappers.join("tool.cmd");
    let bat = fx.wrappers.join("tool.bat");
    std::fs::write(&exact, "exact").expect("exact wrapper should be written");
    std::fs::write(&exe, "exe").expect("exe wrapper should be written");
    Wrapper::cmd(&cmd, "cmd");
    Wrapper::cmd(&bat, "bat");

    for expected in [&exact, &exe, &cmd, &bat] {
        let output = fx.run(&["@which", ":tool"]);
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        Assert::path(stdout.trim(), expected);
        std::fs::remove_file(expected).expect("wrapper candidate should be removed");
    }
}
