#![allow(unused_imports)]
use super::seat::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

#[test]
fn absolute() {
    let fx = Fixture::new();
    let nested = fx.project.join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let wrapper = Wrapper::file(&fx.wrappers, "wrap");
    Wrapper::write(&wrapper, "project");

    let output = bin()
        .current_dir(&nested)
        .env("RUNSEAL_HOME", &fx.home)
        .args(["--profile", "../runseal.toml", "@which", ":wrap"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let printed = Path::new(stdout.trim());
    assert!(printed.is_absolute());
    assert!(
        !printed
            .components()
            .any(|component| { matches!(component, std::path::Component::ParentDir) })
    );
    Assert::path(stdout.trim(), &wrapper);
}

#[test]
fn external() {
    let fx = Fixture::new();

    let output = fx.run(&["@which", "ssh"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("@which currently supports only :wrapper arguments"));
}

#[test]
fn args() {
    let fx = Fixture::new();
    for (args, expected) in [
        (vec!["@"], "internal command name must not be empty"),
        (vec!["@unknown"], "unknown internal command: @unknown"),
        (
            vec!["@profile", "extra"],
            "@profile does not accept arguments",
        ),
        (
            vec!["@wrappers", "extra"],
            "@wrappers does not accept arguments",
        ),
        (
            vec!["@resources", "extra"],
            "@resources does not accept arguments",
        ),
        (
            vec!["@which"],
            "@which requires exactly one :wrapper argument",
        ),
        (
            vec!["@which", ":a", ":b"],
            "@which requires exactly one :wrapper argument",
        ),
        (
            vec!["@resolve"],
            "@resolve requires at least one resource:// URI argument",
        ),
    ] {
        Assert::fails(&fx, &args, expected);
    }
}

#[test]
fn names() {
    let fx = Fixture::new();
    for (args, expected) in [
        (vec![":"], "wrapper name must not be empty"),
        (vec![":.."], "invalid wrapper name: :.."),
        (vec![":bad/name"], "invalid wrapper name: :bad/name"),
        (vec!["@"], "internal command name must not be empty"),
        (vec!["@.."], "invalid internal command name: @.."),
        (
            vec!["@bad/name"],
            "invalid internal command name: @bad/name",
        ),
    ] {
        Assert::fails(&fx, &args, expected);
    }
}

#[cfg(unix)]
#[test]
fn skips() {
    let fx = Fixture::new();
    let source = fx.project.join("source.txt");
    let target = fx.project.join("target.txt");
    std::fs::write(&source, "sealed").expect("source should be written");
    std::fs::write(
        &fx.profile,
        format!(
            r#"
[[injections]]
type = "symlink"
source = "{}"
target = "{}"
cleanup = false
"#,
            source.display(),
            target.display()
        ),
    )
    .expect("profile should be written");

    let output = fx.run(&["@profile"]);

    assert!(output.status.success());
    assert!(!target.exists(), "@profile must not run injections");
}

#[cfg(unix)]
#[test]
fn inert() {
    let fx = Fixture::new();
    let source = fx.project.join("source.txt");
    let target = fx.project.join("target.txt");
    std::fs::write(&source, "sealed").expect("source should be written");
    std::fs::write(
        &fx.profile,
        format!(
            r#"
[[injections]]
type = "symlink"
source = "{}"
target = "{}"
cleanup = false
"#,
            source.display(),
            target.display()
        ),
    )
    .expect("profile should be written");

    let output = fx.run(&["@profile", "--help"]);

    assert!(output.status.success());
    assert!(!target.exists(), "internal help must not run injections");
}

#[cfg(unix)]
#[test]
fn namespace() {
    let temp = TempDir::new().expect("temp dir should be created");
    let path = temp.path().join("bin");
    let profile = temp.path().join("profile.toml");
    std::fs::create_dir_all(&path).expect("bin dir should be created");
    Probe::write(&path.join("profile"));
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
"#,
            path.display()
        ),
    )
    .expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(["profile", "arg"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "arg|");
}
