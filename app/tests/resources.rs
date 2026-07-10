use std::{path::Path, process::Command};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

struct Shell;

impl Shell {
    #[cfg(unix)]
    fn args(script: &str) -> Vec<String> {
        vec!["bash".into(), "--".into(), "-lc".into(), script.into()]
    }

    #[cfg(windows)]
    fn args(script: &str) -> Vec<String> {
        vec![
            "pwsh".into(),
            "--".into(),
            "-NoProfile".into(),
            "-Command".into(),
            script.into(),
        ]
    }
}

struct Script;

impl Script {
    #[cfg(unix)]
    fn env(key: &str) -> String {
        format!("printf '%s' \"${key}\"")
    }

    #[cfg(windows)]
    fn env(key: &str) -> String {
        format!("[Console]::Write($env:{key})")
    }

    #[cfg(unix)]
    fn pair(left: &str, right: &str) -> String {
        format!("printf '%s|%s' \"${left}\" \"${right}\"")
    }

    #[cfg(windows)]
    fn pair(left: &str, right: &str) -> String {
        format!("[Console]::Write(\"$env:{left}|$env:{right}\")")
    }
}

struct Fixture {
    _temp: TempDir,
    project: std::path::PathBuf,
    profile: std::path::PathBuf,
    home: std::path::PathBuf,
}

impl Fixture {
    fn new(text: &str) -> Self {
        let temp = TempDir::new().expect("temp dir should be created");
        let project = temp.path().join("project");
        let profile = project.join("runseal.toml");
        let home = temp.path().join("home");
        std::fs::create_dir_all(&project).expect("project should be created");
        std::fs::write(&profile, text).expect("profile should be written");
        Self {
            _temp: temp,
            project,
            profile,
            home,
        }
    }

    fn resource() -> Self {
        Self::new(
            r#"
[resources]
root = ".resource"

[[injections]]
type = "env"

[injections.vars]
RUNSEAL_RESOURCE_ROOT_A = "resource://"
RUNSEAL_RESOURCE_ROOT_B = "resource://."
RUNSEAL_RESOURCE_A = "resource://local/ssh/config"

[[injections.ops]]
op = "set"
key = "RUNSEAL_RESOURCE_B"
value = "resource://state/export.json"
"#,
        )
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.project)
            .env("RUNSEAL_HOME", &self.home)
            .args(args)
            .output()
            .expect("runseal should run")
    }

    fn profile(&self, args: Vec<String>) -> std::process::Output {
        bin()
            .env("RUNSEAL_HOME", &self.home)
            .arg("--profile")
            .arg(self.profile.to_str().expect("path should be UTF-8"))
            .args(args)
            .output()
            .expect("runseal should run")
    }
}

struct Assert;

impl Assert {
    fn root(value: &str) {
        let path = Path::new(value);
        assert!(path.is_absolute(), "expected {value} to be absolute");
        assert!(
            path.ends_with(Path::new(".resource")),
            "unexpected resource root: {}",
            path.display()
        );
    }

    fn fails(fx: &Fixture, args: &[&str], expected: &str) {
        let output = fx.run(args);
        assert!(!output.status.success(), "{args:?} should fail");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
        assert!(
            stderr.contains(expected),
            "expected stderr for {args:?} to contain {expected:?}, got {stderr:?}"
        );
    }
}

#[test]
fn env() {
    let fx = Fixture::resource();

    let output = fx.profile(Shell::args(&Script::env("RUNSEAL_RESOURCE_ROOT_A")));
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    Assert::root(&stdout);

    let output = fx.profile(Shell::args(&Script::pair(
        "RUNSEAL_RESOURCE_ROOT_B",
        "RUNSEAL_RESOURCE_A",
    )));
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let (left, right) = stdout
        .split_once('|')
        .expect("stdout should include two env values");
    Assert::root(left);
    assert!(
        Path::new(right).ends_with(
            Path::new(".resource")
                .join("local")
                .join("ssh")
                .join("config")
        ),
        "unexpected resource path: {right}"
    );

    let output = fx.profile(Shell::args(&Script::env("RUNSEAL_RESOURCE_B")));
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        Path::new(&stdout).ends_with(Path::new(".resource").join("state").join("export.json")),
        "unexpected resource path: {stdout}"
    );
}

#[test]
fn internal() {
    let fx = Fixture::resource();

    let output = fx.run(&["@profile"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let value = stdout
        .lines()
        .find_map(|line| line.strip_prefix("RUNSEAL_RESOURCE_ROOT="))
        .expect("@profile should include resource root");
    Assert::root(value);

    let output = fx.run(&["@resources"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let value = stdout
        .trim()
        .strip_prefix("RUNSEAL_RESOURCE_ROOT=")
        .expect("output should include resource root");
    Assert::root(value);

    let output = fx.run(&["@resolve", "resource://"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    Assert::root(stdout.trim());

    let output = fx.run(&["@resolve", "resource://", "resource://local/ssh/config"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    Assert::root(lines[0]);
    assert!(
        Path::new(lines[1]).ends_with(
            Path::new(".resource")
                .join("local")
                .join("ssh")
                .join("config")
        ),
        "unexpected resource path: {stdout}"
    );
}

#[test]
fn required() {
    let fx = Fixture::new(
        r#"
[[injections]]
type = "env"

[injections.vars]
RUNSEAL_RESOURCE_A = "resource://local/ssh/config"
"#,
    );

    let output = fx.profile(Shell::args(&Script::env("RUNSEAL_RESOURCE_A")));
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("resource root is not configured"));
    assert!(stderr.contains(fx.profile.to_str().expect("path should be UTF-8")));
}

#[test]
fn invalid() {
    let fx = Fixture::resource();
    for (args, expected) in [
        (
            vec!["@resolve", "local/ssh/config"],
            "expected resource URI to start with resource://",
        ),
        (
            vec!["@resolve", "resource://../secret"],
            "resource URI path must not contain '.' or '..'",
        ),
        (
            vec!["@resolve", "resource://./secret"],
            "resource URI path must not contain '.' or '..'",
        ),
        (
            vec!["@resolve", "resource://local//config"],
            "resource URI path segment must not be empty",
        ),
        (
            vec!["@resolve", "resource://C:/config"],
            "resource URI path segment must not contain ':'",
        ),
    ] {
        Assert::fails(&fx, &args, expected);
    }
}
