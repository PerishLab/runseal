use std::process::Command;

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
    fn profile() -> String {
        "printf '%s|%s' \"$RUNSEAL_TEST_VALUE\" \"$(basename \"$RUNSEAL_PROFILE_PATH\")\"".into()
    }

    #[cfg(windows)]
    fn profile() -> String {
        "[Console]::Write(\"$env:RUNSEAL_TEST_VALUE|$(Split-Path -Leaf $env:RUNSEAL_PROFILE_PATH)\")"
            .into()
    }

    #[cfg(unix)]
    fn symlink(path: &std::path::Path) -> String {
        format!("test -L {}", path.display())
    }
}

struct Probe;

impl Probe {
    #[cfg(unix)]
    fn write(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, "#!/usr/bin/env sh\nprintf '%s|' \"$@\"\n")
            .expect("probe should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("probe metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("probe should be executable");
    }
}

struct Wrapper;

impl Wrapper {
    #[cfg(unix)]
    fn file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        dir.join(format!("{name}.sh"))
    }

    #[cfg(windows)]
    fn file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        dir.join(format!("{name}.cmd"))
    }

    #[cfg(unix)]
    fn name(name: &str) -> String {
        format!("{name}.sh")
    }

    #[cfg(windows)]
    fn name(name: &str) -> String {
        format!("{name}.cmd")
    }

    #[cfg(unix)]
    fn write(path: &std::path::Path, label: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            format!(
                "#!/usr/bin/env sh\nprintf '{}|%s|%s|%s|' \"$1\" \"$RUNSEAL_WRAPPER_NAME\" \"$(basename \"$RUNSEAL_WRAPPER_FILE\")\"\n",
                label
            ),
        )
        .expect("wrapper should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("wrapper metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("wrapper should be executable");
    }

    #[cfg(windows)]
    fn write(path: &std::path::Path, label: &str) {
        std::fs::write(
            path,
            format!(
                "@echo off\r\n<nul set /p=\"{}|%1|%RUNSEAL_WRAPPER_NAME%|%~nx0|\"\r\nexit /b 0\r\n",
                label
            ),
        )
        .expect("wrapper should be written");
    }
}

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
