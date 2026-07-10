use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[cfg(unix)]
fn shell_args(script: &str) -> Vec<String> {
    vec!["bash".into(), "--".into(), "-lc".into(), script.into()]
}

#[cfg(windows)]
fn shell_args(script: &str) -> Vec<String> {
    vec![
        "pwsh".into(),
        "--".into(),
        "-NoProfile".into(),
        "-Command".into(),
        script.into(),
    ]
}

#[cfg(unix)]
fn print_env_script(key: &str) -> String {
    format!("printf '%s' \"${key}\"")
}

#[cfg(windows)]
fn print_env_script(key: &str) -> String {
    format!("[Console]::Write($env:{key})")
}

#[cfg(unix)]
fn explicit_profile_script() -> String {
    "printf '%s|%s' \"$RUNSEAL_TEST_VALUE\" \"$(basename \"$RUNSEAL_PROFILE_PATH\")\"".into()
}

#[cfg(windows)]
fn explicit_profile_script() -> String {
    "[Console]::Write(\"$env:RUNSEAL_TEST_VALUE|$(Split-Path -Leaf $env:RUNSEAL_PROFILE_PATH)\")"
        .into()
}

#[cfg(unix)]
fn symlink_check_script(path: &std::path::Path) -> String {
    format!("test -L {}", path.display())
}

#[cfg(unix)]
fn make_probe(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, "#!/usr/bin/env sh\nprintf '%s|' \"$@\"\n")
        .expect("probe should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("probe metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("probe should be executable");
}

#[cfg(unix)]
fn wrapper_file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    dir.join(format!("{name}.sh"))
}

#[cfg(windows)]
fn wrapper_file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    dir.join(format!("{name}.cmd"))
}

#[cfg(unix)]
fn wrapper_basename(name: &str) -> String {
    format!("{name}.sh")
}

#[cfg(windows)]
fn wrapper_basename(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(unix)]
fn make_wrapper(path: &std::path::Path, label: &str) {
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
fn make_wrapper(path: &std::path::Path, label: &str) {
    std::fs::write(
        path,
        format!(
            "@echo off\r\n<nul set /p=\"{}|%1|%RUNSEAL_WRAPPER_NAME%|%~nx0|\"\r\nexit /b 0\r\n",
            label
        ),
    )
    .expect("wrapper should be written");
}

#[test]
fn help_without_command() {
    let output = bin().output().expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--profile"));
}

#[test]
fn explicit_profile_runs() {
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
        .args(shell_args(&explicit_profile_script()))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "from-toml|profile.toml");
}

#[test]
fn cwd_beats_home() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work");
    let runseal_home = temp.path().join("home");
    let profile_home = runseal_home.join("profiles");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&profile_home).expect("profile home should be created");
    std::fs::write(
        cwd.join("runseal.yaml"),
        "injections:\n  - type: env\n    vars:\n      PICKED: cwd\n",
    )
    .expect("cwd profile should be written");
    std::fs::write(
        profile_home.join("default.toml"),
        "[[injections]]\ntype = \"env\"\n[injections.vars]\nPICKED = \"home\"\n",
    )
    .expect("default profile should be written");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", &runseal_home)
        .args(shell_args(&print_env_script("PICKED")))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "cwd");
}

#[cfg(unix)]
#[test]
fn symlink_lifecycle() {
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
        .args(shell_args(&symlink_check_script(&target)))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    assert!(!target.exists(), "symlink should be cleaned after command");
}

#[cfg(unix)]
#[test]
fn symlink_shutdown_contention() {
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
        .args(shell_args(&format!("rm -- '{}'", target.display())))
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
fn argv_injection_prefixes_command() {
    let temp = TempDir::new().expect("temp dir should be created");
    let bin_dir = temp.path().join("bin");
    let profile = temp.path().join("profile.toml");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    make_probe(&bin_dir.join("probe"));
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
            bin_dir.display()
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
fn wrapper_uses_profile_root() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let cwd = project.join("nested");
    let project_wrappers = project.join(".runseal/wrappers");
    let home_wrappers = temp.path().join("home/wrappers");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");
    std::fs::create_dir_all(&project_wrappers).expect("project wrappers should be created");
    std::fs::create_dir_all(&home_wrappers).expect("home wrappers should be created");
    std::fs::write(project.join("runseal.toml"), "injections = []\n")
        .expect("profile should be written");
    make_wrapper(&wrapper_file(&project_wrappers, "wrap"), "project");
    make_wrapper(&wrapper_file(&home_wrappers, "wrap"), "home");

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
        format!("project|arg|wrap|{}|", wrapper_basename("wrap"))
    );
}

#[test]
fn wrapper_missing_lists_paths() {
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
fn child_exit_code() {
    let temp = TempDir::new().expect("temp dir should be created");
    let profile = temp.path().join("profile.json");
    std::fs::write(&profile, r#"{"injections":[]}"#).expect("profile should be written");

    let output = bin()
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("--profile")
        .arg(profile.to_str().expect("path should be UTF-8"))
        .args(shell_args("exit 17"))
        .output()
        .expect("runseal should run");

    assert_eq!(output.status.code(), Some(17));
}

#[test]
fn toml_beats_yaml() {
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
        .args(shell_args(&print_env_script("PICKED")))
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "toml");
}

#[test]
fn missing_profile_paths() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("work");
    let home = temp.path().join("home");
    std::fs::create_dir_all(&cwd).expect("cwd should be created");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", &home)
        .args(shell_args("true"))
        .output()
        .expect("runseal should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("runseal.toml"));
    assert!(stderr.contains("default.json"));
}
