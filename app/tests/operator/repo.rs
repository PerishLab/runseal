#![cfg(unix)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    project: PathBuf,
    bin: PathBuf,
}

fn fixture() -> Option<Fixture> {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let bin = temp.path().join("bin");
    std::fs::create_dir_all(&project).expect("project should be created");
    std::fs::create_dir_all(&bin).expect("stub bin dir should be created");
    write_stub(
        &bin.join("git"),
        r#"#!/usr/bin/env sh
set -eu
case "${1:-}" in
  --version)
    ;;
  branch)
    if [ "${2:-}" = "--show-current" ]; then
      printf '%s\n' "${RUNSEAL_TEST_BRANCH:-feat/deno}"
    elif [ "${2:-}" = "-D" ]; then
      printf 'git %s\n' "$*" >> "${RUNSEAL_TEST_LOG:?}"
    else
      exit 9
    fi
    ;;
  status)
    if [ "${2:-}" = "--short" ]; then
      printf '%s\n' "${RUNSEAL_TEST_STATUS:-}"
    else
      printf 'git %s\n' "$*" >> "${RUNSEAL_TEST_LOG:?}"
    fi
    ;;
  remote)
    [ "${2:-}" = "get-url" ] || exit 9
    [ "${3:-}" = "origin" ] || exit 9
    printf '%s\n' "${RUNSEAL_TEST_REMOTE_ORIGIN:-git@git.perish.top:PerishFire/runseal.git}"
    ;;
  rev-parse)
    if [ "${2:-}" = "--verify" ]; then
      exit "${RUNSEAL_TEST_REV_PARSE_STATUS:-0}"
    fi
    printf '%s\n' "${RUNSEAL_TEST_REF_SHA:-abc123}"
    ;;
  merge-base)
    exit "${RUNSEAL_TEST_MERGE_BASE_STATUS:-0}"
    ;;
  rev-list)
    [ "${2:-}" = "--count" ] || exit 9
    printf '%s\n' "${RUNSEAL_TEST_AHEAD:-1}"
    ;;
  log)
    printf '%s\n' "${RUNSEAL_TEST_LOG_SUBJECTS:-ops: add land wrapper}"
    ;;
  *)
    printf 'git %s\n' "$*" >> "${RUNSEAL_TEST_LOG:?}"
    ;;
esac
"#,
    );
    write_stub(
        &bin.join("runseal"),
        r#"#!/usr/bin/env sh
set -eu
printf 'runseal %s\n' "$*" >> "${RUNSEAL_TEST_LOG:?}"
[ "${1:-}" = "@tool" ] || exit 9
[ "${2:-}" = "forgejo" ] || exit 9
case "${3:-}:${4:-}" in
  pr:find)
    if [ "${RUNSEAL_TEST_PR_FIND+x}" ]; then
      printf '%s\n' "$RUNSEAL_TEST_PR_FIND"
    else
      printf '%s\n' '{"number":42,"html_url":"https://git.test/pull/42"}'
    fi
    ;;
  pr:create)
    if [ "${RUNSEAL_TEST_PR_CREATE+x}" ]; then
      printf '%s\n' "$RUNSEAL_TEST_PR_CREATE"
    else
      printf '%s\n' '{"number":77,"html_url":"https://git.test/pull/77"}'
    fi
    ;;
  pr:guard)
    printf '%s\n' '{"id":8,"status":"success","commit_sha":"guarded123"}'
    ;;
  pr:merge)
    ;;
  workflow:dispatch)
    if [ "${RUNSEAL_TEST_DISPATCH+x}" ]; then
      printf '%s\n' "$RUNSEAL_TEST_DISPATCH"
    else
      printf '%s\n' '{"id":12345,"run_number":9}'
    fi
    ;;
  run:watch)
    ;;
  *)
    exit 9
    ;;
esac
"#,
    );
    Some(Fixture {
        _temp: temp,
        project,
        bin,
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("app dir should have repo parent")
        .to_path_buf()
}

fn write_stub(path: &Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, content).expect("stub should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("stub metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("stub should be executable");
}

fn run_active_wrapper(fx: &Fixture, name: &str, args: &[&str]) -> std::process::Output {
    run_wrapper_env(fx, name, args, &[])
}

fn run_wrapper_env(
    fx: &Fixture,
    name: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> std::process::Output {
    let log = fx.project.join("commands.log");
    let path = prepend_path(&fx.bin);
    Command::new(env!("CARGO_BIN_EXE_runseal"))
        .current_dir(&fx.project)
        .env("PATH", path)
        .env("RUNSEAL_TEST_LOG", &log)
        .arg("-p")
        .arg(repo_root().join("runseal.toml"))
        .arg(format!(":{name}"))
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .expect("active operator wrapper should run")
}

fn prepend_path(first: &Path) -> OsString {
    let mut paths = vec![first.to_path_buf()];
    if let Some(runseal_dir) = Path::new(env!("CARGO_BIN_EXE_runseal")).parent() {
        paths.push(runseal_dir.to_path_buf());
    }
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("PATH should be joinable")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn command_log(fx: &Fixture) -> String {
    std::fs::read_to_string(fx.project.join("commands.log")).unwrap_or_default()
}

#[test]
fn land_help_option() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(&fx, "land", &["--help"]);

    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(stdout.contains("Usage: runseal :land [options]"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn land_dry_run_matches() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(&fx, "land", &["--dry-run"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "\
[dry-run] would run:
  git fetch origin main
  verify feat/deno is clean, not main, contains origin/main, ahead >= 1
  git push -u origin feat/deno
  runseal @tool forgejo pr find --repo PerishFire/runseal --head feat/deno --base main
  runseal @tool forgejo pr create --repo PerishFire/runseal --base main --head feat/deno --title <commit>  # if missing
  runseal @tool forgejo pr guard --repo PerishFire/runseal --number <n>
  runseal @tool forgejo pr merge --repo PerishFire/runseal --number <n> --head <guarded-sha> --delete-branch true
  git checkout main
  git pull --ff-only origin main
  git branch -D feat/deno  # if still present locally
"
    );
}

#[test]
fn land_rejects_dirty_tree() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "land",
        &["--dry-run"],
        &[("RUNSEAL_TEST_STATUS", " M README.md")],
    );

    assert!(!output.status.success());
    assert!(stderr(&output).contains("land: working tree must be clean"));
}

#[test]
fn land_rejects_base_branch() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "land",
        &["--dry-run"],
        &[("RUNSEAL_TEST_BRANCH", "main")],
    );

    assert!(!output.status.success());
    assert!(stderr(&output).contains("land: must run on a topic branch, not main"));
}

#[test]
fn land_reuses_open_pr() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "land",
        &["--no-delete"],
        &[(
            "RUNSEAL_TEST_PR_FIND",
            r#"{"number":42,"html_url":"https://git.test/pull/42"}"#,
        )],
    );

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "\
https://git.test/pull/42
"
    );
    assert_eq!(
        command_log(&fx),
        "\
git fetch origin main
git push -u origin feat/deno
runseal @tool forgejo pr find --repo PerishFire/runseal --head feat/deno --base main
runseal @tool forgejo pr guard --repo PerishFire/runseal --number 42
runseal @tool forgejo pr merge --repo PerishFire/runseal --number 42 --head guarded123 --delete-branch false
git checkout main
git pull --ff-only origin main
"
    );
}

#[test]
fn land_creates_and_merges() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "land",
        &["--body", "body text", "--base", "develop"],
        &[
            ("RUNSEAL_TEST_PR_FIND", "null"),
            (
                "RUNSEAL_TEST_PR_CREATE",
                r#"{"number":77,"html_url":"https://git.test/pull/77"}"#,
            ),
            ("RUNSEAL_TEST_LOG_SUBJECTS", "ops: add land wrapper"),
        ],
    );

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "\
https://git.test/pull/77
"
    );
    assert_eq!(
        command_log(&fx),
        "\
git fetch origin develop
git push -u origin feat/deno
runseal @tool forgejo pr find --repo PerishFire/runseal --head feat/deno --base develop
runseal @tool forgejo pr create --repo PerishFire/runseal --base develop --head feat/deno --title ops: add land wrapper --body body text
runseal @tool forgejo pr guard --repo PerishFire/runseal --number 77
runseal @tool forgejo pr merge --repo PerishFire/runseal --number 77 --head guarded123 --delete-branch true
git checkout develop
git pull --ff-only origin develop
git branch -D feat/deno
"
    );
}

#[test]
fn release_help_without_args() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(&fx, "release", &[]);

    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(stdout.contains("Usage: runseal :release --channel=stable|beta [options]"));
    assert!(stdout.contains("--watch"));
}

#[test]
fn release_dry_run_matches() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(
        &fx,
        "release",
        &[
            "--channel",
            "beta",
            "--ref",
            "feature/ref",
            "--version",
            "v1.2.3-beta.4",
            "--dry-run",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "runseal @tool forgejo workflow dispatch --repo PerishFire/runseal --workflow release-beta.yml --ref feature/ref --input version_override=v1.2.3-beta.4\n"
    );
}

#[test]
fn release_requires_channel() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(&fx, "release", &["--dry-run"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("release: --channel is required"));
}

#[test]
fn release_rejects_invalid_channel() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_active_wrapper(&fx, "release", &["--channel", "nightly", "--dry-run"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("invalid choice"));
}

#[test]
fn release_watches_trigger_url() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "release",
        &["--channel", "stable", "--watch"],
        &[("RUNSEAL_TEST_DISPATCH", r#"{"id":12345,"run_number":9}"#)],
    );

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "\
triggered release-stable.yml run 12345 for ref main
"
    );
    assert_eq!(
        command_log(&fx),
        "\
runseal @tool forgejo workflow dispatch --repo PerishFire/runseal --workflow release-stable.yml --ref main --input version_override=
runseal @tool forgejo run watch --repo PerishFire/runseal --id 12345 --interval 10
"
    );
}

#[test]
fn release_uses_latest_run() {
    let Some(fx) = fixture() else {
        return;
    };

    let output = run_wrapper_env(
        &fx,
        "release",
        &["--channel", "beta", "--ref", "feature/ref", "--watch"],
        &[("RUNSEAL_TEST_DISPATCH", r#"{"id":67890,"run_number":10}"#)],
    );

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "triggered release-beta.yml run 67890 for ref feature/ref\n"
    );
    assert_eq!(
        command_log(&fx),
        "\
runseal @tool forgejo workflow dispatch --repo PerishFire/runseal --workflow release-beta.yml --ref feature/ref --input version_override=
runseal @tool forgejo run watch --repo PerishFire/runseal --id 67890 --interval 10
"
    );
}
