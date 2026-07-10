use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
    command.env_remove("RUNSEAL_PROFILE_HOME");
    command.env_remove("RUNSEAL_PROFILE_PATH");
    command
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
}

struct Profile;

impl Profile {
    fn text(value: &str) -> String {
        format!("[[injections]]\ntype = \"env\"\n[injections.vars]\nPICKED = \"{value}\"\n")
    }

    fn picked(cwd: &std::path::Path, home: &std::path::Path) -> String {
        let output = bin()
            .current_dir(cwd)
            .env("RUNSEAL_HOME", home)
            .args(Shell::args(&Script::env("PICKED")))
            .output()
            .expect("runseal should run");

        assert!(output.status.success());
        String::from_utf8(output.stdout).expect("stdout should be UTF-8")
    }
}

#[test]
fn explicit() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let nested = project.join("nested");
    let profile = project.join("runseal.toml");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    std::fs::write(&profile, "injections = []\n").expect("profile should be written");

    let output = bin()
        .current_dir(&nested)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .args(["--profile", "../runseal.toml"])
        .args(Shell::args(&Script::env("RUNSEAL_PROFILE_PATH")))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let printed = std::path::Path::new(stdout.as_str());
    assert!(printed.is_absolute());
    assert!(
        !printed
            .components()
            .any(|component| { matches!(component, std::path::Component::ParentDir) })
    );
    assert!(printed.ends_with("runseal.toml"));
}

#[test]
fn ancestor() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let nested = project.join("a/b/c");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    std::fs::write(project.join("runseal.toml"), Profile::text("parent"))
        .expect("parent profile should be written");

    assert_eq!(
        Profile::picked(&nested, &temp.path().join("home")),
        "parent"
    );
}

#[test]
fn nearest() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let nested = project.join("a/b/c");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    std::fs::write(project.join("runseal.toml"), Profile::text("parent"))
        .expect("parent profile should be written");
    std::fs::write(
        nested.join("runseal.yaml"),
        "injections:\n  - type: env\n    vars:\n      PICKED: nested\n",
    )
    .expect("nested profile should be written");

    assert_eq!(
        Profile::picked(&nested, &temp.path().join("home")),
        "nested"
    );
}

#[test]
fn priority() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let nested = project.join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    std::fs::write(
        nested.join("runseal.yaml"),
        "injections:\n  - type: env\n    vars:\n      PICKED: nested-yaml\n",
    )
    .expect("nested yaml should be written");
    std::fs::write(project.join("runseal.toml"), Profile::text("parent-toml"))
        .expect("parent toml should be written");
    std::fs::write(nested.join("runseal.toml"), Profile::text("nested-toml"))
        .expect("nested toml should be written");

    assert_eq!(
        Profile::picked(&nested, &temp.path().join("home")),
        "nested-toml"
    );
}

#[test]
fn fallback() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work/a/b");
    let home = temp.path().join("home");
    let profiles = home.join("profiles");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&profiles).expect("profile home should be created");
    std::fs::write(profiles.join("default.toml"), Profile::text("home"))
        .expect("default profile should be written");

    assert_eq!(Profile::picked(&cwd, &home), "home");
}
