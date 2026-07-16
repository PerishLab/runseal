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

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should be created");
        let project = temp.path().join("project");
        let bin = temp.path().join("bin");
        std::fs::create_dir_all(&project).expect("project should be created");
        std::fs::create_dir_all(&bin).expect("stub bin dir should be created");
        Stub::write(
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
        Stub::write(
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
        Self {
            _temp: temp,
            project,
            bin,
        }
    }

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app dir should have repo parent")
            .to_path_buf()
    }

    fn run(&self, name: &str, args: &[&str]) -> std::process::Output {
        self.env(name, args, &[])
    }

    fn env(&self, name: &str, args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
        let log = self.project.join("commands.log");
        Command::new(env!("CARGO_BIN_EXE_runseal"))
            .current_dir(&self.project)
            .env("PATH", Paths::prepend(&self.bin))
            .env("RUNSEAL_TEST_LOG", &log)
            .arg("-p")
            .arg(Self::root().join("runseal.toml"))
            .arg(format!(":{name}"))
            .args(args)
            .envs(envs.iter().copied())
            .output()
            .expect("active operator wrapper should run")
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.project.join("commands.log")).unwrap_or_default()
    }
}

struct Stub;

impl Stub {
    fn write(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, content).expect("stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("stub should be executable");
    }
}

struct Paths;

impl Paths {
    fn prepend(first: &Path) -> OsString {
        let mut paths = vec![first.to_path_buf()];
        if let Some(dir) = Path::new(env!("CARGO_BIN_EXE_runseal")).parent() {
            paths.push(dir.to_path_buf());
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

mod release {
    use super::*;

    #[test]
    fn help() {
        let fx = Fixture::new();

        let output = fx.run("release", &[]);

        assert!(output.status.success());
        let stdout = stdout(&output);
        assert!(stdout.contains("Usage: runseal :release --channel=stable|beta [options]"));
        assert!(stdout.contains("--watch"));
    }

    #[test]
    fn dry() {
        let fx = Fixture::new();

        let output = fx.run(
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
    fn channel() {
        let fx = Fixture::new();

        let output = fx.run("release", &["--dry-run"]);

        assert!(!output.status.success());
        assert!(stderr(&output).contains("release: --channel is required"));
    }

    #[test]
    fn invalid() {
        let fx = Fixture::new();

        let output = fx.run("release", &["--channel", "nightly", "--dry-run"]);

        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains("invalid choice"));
    }

    #[test]
    fn watch() {
        let fx = Fixture::new();

        let output = fx.env(
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
            fx.log(),
            "\
runseal @tool forgejo workflow dispatch --repo PerishFire/runseal --workflow release-stable.yml --ref main --input version_override=
runseal @tool forgejo run watch --repo PerishFire/runseal --id 12345 --interval 10
"
        );
    }

    #[test]
    fn latest() {
        let fx = Fixture::new();

        let output = fx.env(
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
            fx.log(),
            "\
runseal @tool forgejo workflow dispatch --repo PerishFire/runseal --workflow release-beta.yml --ref feature/ref --input version_override=
runseal @tool forgejo run watch --repo PerishFire/runseal --id 67890 --interval 10
"
        );
    }
}
