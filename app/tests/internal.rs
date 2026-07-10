use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[cfg(unix)]
fn wrapper_file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.sh"))
}

#[cfg(windows)]
fn wrapper_file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.cmd"))
}

#[cfg(unix)]
fn make_wrapper(path: &Path, label: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, format!("#!/usr/bin/env sh\nprintf '{}'\n", label))
        .expect("wrapper should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("wrapper metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("wrapper should be executable");
}

#[cfg(windows)]
fn make_wrapper(path: &Path, label: &str) {
    make_cmdlike(path, label);
}

#[cfg(windows)]
fn make_cmdlike(path: &Path, label: &str) {
    std::fs::write(
        path,
        format!("@echo off\r\n<nul set /p=\"{}\"\r\nexit /b 0\r\n", label),
    )
    .expect("wrapper should be written");
}

#[cfg(unix)]
fn make_probe(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, "#!/usr/bin/env sh\nprintf '%s|' \"$@\"\n")
        .expect("probe should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("probe metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("probe should be executable");
}

struct Fixture {
    _temp: TempDir,
    project: PathBuf,
    profile: PathBuf,
    home: PathBuf,
    project_wrappers: PathBuf,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let profile = project.join("runseal.toml");
    let home = temp.path().join("home");
    let project_wrappers = project.join(".runseal").join("wrappers");
    std::fs::create_dir_all(&project_wrappers).expect("project wrappers should be created");
    std::fs::create_dir_all(home.join("wrappers")).expect("home wrappers should be created");
    std::fs::write(
        &profile,
        "injections = []\n[resources]\nroot = \".resource\"\n",
    )
    .expect("profile should be written");
    Fixture {
        _temp: temp,
        project,
        profile,
        home,
        project_wrappers,
    }
}

fn run_in(fx: &Fixture, args: &[&str]) -> std::process::Output {
    bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .args(args)
        .output()
        .expect("runseal should run")
}

fn path_suffix(path: &Path, count: usize) -> PathBuf {
    path.components()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn assert_path_ends_with(actual: &str, expected: &Path) {
    let expected_suffix = path_suffix(expected, 4);
    assert!(
        Path::new(actual).ends_with(&expected_suffix),
        "expected {actual:?} to end with {}",
        expected_suffix.display()
    );
}

fn assert_fails(fx: &Fixture, args: &[&str], expected: &str) {
    let output = run_in(fx, args);
    assert!(!output.status.success(), "{args:?} should fail");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        stderr.contains(expected),
        "expected stderr for {args:?} to contain {expected:?}, got {stderr:?}"
    );
}

#[test]
fn help_explains_model() {
    let output = bin().arg("--help").output().expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("runseal <cmd>"));
    assert!(stdout.contains("runseal :<name>"));
    assert!(stdout.contains("runseal @<name>"));
    assert!(stdout.contains(".ts files are run with deno"));
    assert!(stdout.contains("runseal @tool for atomic glue"));
    assert!(stdout.contains("@profile"));
    assert!(stdout.contains("@resolve"));
    assert!(stdout.contains("current directory upward"));
    assert!(stdout.contains("https://git.perish.top/PerishFire/runseal"));
}

#[test]
fn internal_help_topics() {
    let fx = fixture();
    for (args, expected) in [
        (vec!["@profile", "--help"], "Usage: runseal @profile"),
        (vec!["@profile", "-h"], "RUNSEAL_PROFILE_PATH"),
        (vec!["@profile", "help"], "Profile discovery"),
        (vec!["@resources", "--help"], "Usage: runseal @resources"),
        (vec!["@resolve", "--help"], "Usage: runseal @resolve"),
        (vec!["@tool", "--help"], "Usage: runseal @tool"),
        (vec!["@wrappers", "--help"], "Lookup order"),
        (
            vec!["@wrappers", "-h"],
            ".ts wrappers for structured cross-platform operations",
        ),
        (vec!["@which", "--help"], "Usage: runseal @which :<wrapper>"),
    ] {
        let output = run_in(&fx, &args);
        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert!(
            stdout.contains(expected),
            "expected stdout for {args:?} to contain {expected:?}, got {stdout:?}"
        );
    }
}

#[test]
fn unknown_help_fails() {
    let fx = fixture();

    assert_fails(
        &fx,
        &["@unknown", "--help"],
        "unknown internal command: @unknown",
    );
}

#[test]
fn profile_prints_paths() {
    let fx = fixture();
    std::fs::write(&fx.profile, "not valid profile toml").expect("profile should be written");

    let output = run_in(&fx, &["@profile"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("RUNSEAL_HOME="));
    assert!(stdout.contains("RUNSEAL_PROFILE_HOME="));
    assert!(stdout.contains("RUNSEAL_PROFILE_PATH="));
    assert!(stdout.contains("RUNSEAL_WRAPPER_PATH="));
    assert!(stdout.contains(fx.profile.to_str().expect("path should be UTF-8")));
}

#[test]
fn which_resolves_wrapper() {
    let fx = fixture();
    let wrapper = wrapper_file(&fx.project_wrappers, "wrap");
    make_wrapper(&wrapper, "project");

    let output = run_in(&fx, &["@which", ":wrap"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_path_ends_with(stdout.trim(), &wrapper);
}

#[test]
fn which_prints_absolute_path() {
    let fx = fixture();
    let nested = fx.project.join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir should be created");
    let wrapper = wrapper_file(&fx.project_wrappers, "wrap");
    make_wrapper(&wrapper, "project");

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
    assert_path_ends_with(stdout.trim(), &wrapper);
}

#[test]
fn which_rejects_external() {
    let fx = fixture();

    let output = run_in(&fx, &["@which", "ssh"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("@which currently supports only :wrapper arguments"));
}

#[test]
fn internal_rejects_args() {
    let fx = fixture();
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
        assert_fails(&fx, &args, expected);
    }
}

#[test]
fn names_reject_invalid() {
    let fx = fixture();
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
        assert_fails(&fx, &args, expected);
    }
}

#[cfg(unix)]
#[test]
fn profile_skips_injections() {
    let fx = fixture();
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

    let output = run_in(&fx, &["@profile"]);

    assert!(output.status.success());
    assert!(!target.exists(), "@profile must not run injections");
}

#[cfg(unix)]
#[test]
fn internal_help_skips_injections() {
    let fx = fixture();
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

    let output = run_in(&fx, &["@profile", "--help"]);

    assert!(output.status.success());
    assert!(!target.exists(), "internal help must not run injections");
}

#[cfg(windows)]
#[test]
fn windows_ext_priority() {
    let fx = fixture();
    let exact = fx.project_wrappers.join("tool");
    let exe = fx.project_wrappers.join("tool.exe");
    let cmd = fx.project_wrappers.join("tool.cmd");
    let bat = fx.project_wrappers.join("tool.bat");
    std::fs::write(&exact, "exact").expect("exact wrapper should be written");
    std::fs::write(&exe, "exe").expect("exe wrapper should be written");
    make_cmdlike(&cmd, "cmd");
    make_cmdlike(&bat, "bat");

    for expected in [&exact, &exe, &cmd, &bat] {
        let output = run_in(&fx, &["@which", ":tool"]);
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert_path_ends_with(stdout.trim(), expected);
        std::fs::remove_file(expected).expect("wrapper candidate should be removed");
    }
}

#[cfg(windows)]
#[test]
fn windows_cmd_exec() {
    let fx = fixture();
    let cmd = fx.project_wrappers.join("tool.cmd");
    let bat = fx.project_wrappers.join("tool.bat");
    make_cmdlike(&cmd, "cmd");
    make_cmdlike(&bat, "bat");

    let which = run_in(&fx, &["@which", ":tool"]);
    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    assert_path_ends_with(stdout.trim(), &cmd);

    let output = run_in(&fx, &[":tool"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "cmd");
}

#[cfg(unix)]
#[test]
fn namespace_keeps_external() {
    let temp = TempDir::new().expect("temp dir should be created");
    let bin_dir = temp.path().join("bin");
    let profile = temp.path().join("profile.toml");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    make_probe(&bin_dir.join("profile"));
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
            bin_dir.display()
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
