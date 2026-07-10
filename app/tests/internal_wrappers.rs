#![cfg(unix)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

fn shell_wrapper_file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.sh"))
}

fn ts_wrapper_file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.ts"))
}

fn make_shell_wrapper(path: &Path, label: &str) {
    write_executable(path, &format!("#!/usr/bin/env sh\nprintf '{}\\n'\n", label));
}

fn make_ts_wrapper(path: &Path) {
    std::fs::write(path, "console.log(Deno.args.join('|'));\n")
        .expect("ts wrapper should be written");
}

fn write_executable(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, content).expect("executable should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("executable metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("executable should be executable");
}

struct Fixture {
    _temp: TempDir,
    project: PathBuf,
    home: PathBuf,
    bin: PathBuf,
    project_wrappers: PathBuf,
    home_wrappers: PathBuf,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let home = temp.path().join("home");
    let bin = temp.path().join("bin");
    let project_wrappers = project.join(".runseal").join("wrappers");
    let home_wrappers = home.join("wrappers");
    std::fs::create_dir_all(project.join(".runseal")).expect("project .runseal should exist");
    std::fs::create_dir_all(&project_wrappers).expect("project wrappers should be created");
    std::fs::create_dir_all(&home_wrappers).expect("home wrappers should be created");
    std::fs::create_dir_all(&bin).expect("stub bin should be created");
    std::fs::write(project.join(".runseal/deno.json"), "{}\n")
        .expect("deno config should be written");
    std::fs::write(
        project.join("runseal.toml"),
        r#"
injections = []

[resources]
root = ".resource"

[deno]
config = ".runseal/deno.json"
lock = ".runseal/deno.lock"
permissions = ["--allow-env"]
"#,
    )
    .expect("profile should be written");
    Fixture {
        _temp: temp,
        project,
        home,
        bin,
        project_wrappers,
        home_wrappers,
    }
}

fn run_in(fx: &Fixture, args: &[&str]) -> std::process::Output {
    bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", prepend_path(&fx.bin))
        .args(args)
        .output()
        .expect("runseal should run")
}

fn prepend_path(first: &Path) -> OsString {
    let mut paths = vec![first.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("PATH should be joinable")
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

fn install_fake_deno(fx: &Fixture) -> PathBuf {
    let log = fx.project.join("deno.log");
    write_executable(
        &fx.bin.join("deno"),
        r#"#!/usr/bin/env sh
set -eu
printf 'deno %s\n' "$*" >> "${RUNSEAL_TEST_DENO_LOG:?}"
printf 'name=%s\n' "${RUNSEAL_WRAPPER_NAME:-}"
printf 'file=%s\n' "${RUNSEAL_WRAPPER_FILE:-}"
printf 'args=%s\n' "$*"
"#,
    );
    log
}

#[test]
fn wrappers_show_effective() {
    let fx = fixture();
    make_ts_wrapper(&ts_wrapper_file(&fx.project_wrappers, "wrap"));
    make_shell_wrapper(&shell_wrapper_file(&fx.home_wrappers, "wrap"), "home");
    make_ts_wrapper(&ts_wrapper_file(&fx.home_wrappers, "home-only"));

    let output = run_in(&fx, &["@wrappers"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(":wrap"));
    assert!(stdout.contains(":home-only"));
    assert!(stdout.contains("profile"));
    assert!(stdout.contains("home"));
    let wrap_line = stdout
        .lines()
        .find(|line| line.contains(":wrap"))
        .expect("wrap should be listed");
    let wrap_file = wrap_line
        .split_whitespace()
        .last()
        .expect("wrap line should include a file");
    assert!(wrap_line.contains("profile"));
    assert!(
        Path::new(wrap_file).ends_with(path_suffix(
            &ts_wrapper_file(&fx.project_wrappers, "wrap"),
            4
        )),
        "expected {wrap_file} to point at the profile wrapper"
    );
}

#[test]
fn ts_wrapper_resolves() {
    let fx = fixture();
    let wrapper = ts_wrapper_file(&fx.project_wrappers, "tool");
    make_ts_wrapper(&wrapper);

    let which = run_in(&fx, &["@which", ":tool"]);

    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    assert_path_ends_with(stdout.trim(), &wrapper);
}

#[test]
fn extensionless_is_ignored() {
    let fx = fixture();
    make_shell_wrapper(&fx.project_wrappers.join("legacy"), "legacy");

    let output = run_in(&fx, &[":legacy"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("wrapper not found: :legacy"));
    assert!(stderr.contains("legacy.ts"));
    assert!(stderr.contains("legacy.sh"));
    assert!(!stderr.contains(".runseal/wrappers/legacy\n"));
}

#[test]
fn deno_uses_profile_policy() {
    let fx = fixture();
    let log = install_fake_deno(&fx);
    let wrapper = ts_wrapper_file(&fx.project_wrappers, "tool");
    make_ts_wrapper(&wrapper);

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", prepend_path(&fx.bin))
        .env("RUNSEAL_TEST_DENO_LOG", &log)
        .args([":tool", "hello", "world"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("name=tool"));
    assert_path_ends_with(
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

#[test]
fn deno_expands_permission_env() {
    let fx = fixture();
    let log = install_fake_deno(&fx);
    let wrapper = ts_wrapper_file(&fx.project_wrappers, "tool");
    make_ts_wrapper(&wrapper);
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
        .env("PATH", prepend_path(&fx.bin))
        .args([":tool"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let log = std::fs::read_to_string(log).expect("deno log should be readable");
    assert!(log.contains(&format!("--allow-read={}", allowed.display())));
    assert!(!log.contains("${RUNSEAL_TEST_READ}"));
}

#[test]
fn deno_requires_profile_policy() {
    let fx = fixture();
    std::fs::write(
        fx.project.join("runseal.toml"),
        "injections = []\n[resources]\nroot = \".resource\"\n",
    )
    .expect("profile should be written");
    make_ts_wrapper(&ts_wrapper_file(&fx.project_wrappers, "tool"));

    let output = run_in(&fx, &[":tool"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("deno wrapper requires a [deno] profile policy"));
}

#[test]
fn ts_shadows_shell() {
    let fx = fixture();
    let log = install_fake_deno(&fx);
    make_ts_wrapper(&ts_wrapper_file(&fx.project_wrappers, "tool"));
    make_shell_wrapper(&shell_wrapper_file(&fx.project_wrappers, "tool"), "shell");

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", prepend_path(&fx.bin))
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
fn shell_runs_without_ts() {
    let fx = fixture();
    make_shell_wrapper(&shell_wrapper_file(&fx.project_wrappers, "tool"), "shell");

    let output = run_in(&fx, &[":tool"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "shell\n");
}

#[test]
fn wrappers_hide_shadow() {
    let fx = fixture();
    let project_wrapper = ts_wrapper_file(&fx.project_wrappers, "wrap");
    make_ts_wrapper(&project_wrapper);
    make_shell_wrapper(&shell_wrapper_file(&fx.home_wrappers, "wrap"), "home");

    let which = run_in(&fx, &["@which", ":wrap"]);
    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    assert_path_ends_with(stdout.trim(), &project_wrapper);

    let output = run_in(&fx, &["@wrappers"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let wrap_lines = stdout
        .lines()
        .filter(|line| line.starts_with(":wrap "))
        .collect::<Vec<_>>();
    assert_eq!(wrap_lines.len(), 1);
    assert!(wrap_lines[0].contains("profile"));
}
