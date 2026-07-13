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

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    let bin = temp.path().join("bin");
    std::fs::create_dir_all(&project).expect("project should be created");
    std::fs::create_dir_all(&bin).expect("bin should be created");
    Command::new("git")
        .arg("init")
        .arg(&project)
        .output()
        .expect("git init should run");
    Fixture::write(&project);
    Git::write(&bin.join("git"));
    Script::write(&bin.join("cargo"));
    Script::write(&bin.join("negentropy"));
    Script::write(&bin.join("sh"));
    Script::write(&bin.join("bash"));
    Script::write(&bin.join("sed"));
    Script::write(&bin.join("grep"));
    Fixture {
        _temp: temp,
        project,
        bin,
    }
}

impl Fixture {
    fn write(project: &Path) {
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "negentropy.toml",
            "vocabulary.toml",
            "docs/vocabulary.md",
            "manage.sh",
            "runseal.toml",
            ".runseal/deno.json",
            ".runseal/deno.lock",
            ".runseal/negentropy.version",
            ".runseal/hooks/pre-commit",
            ".runseal/hooks/commit-msg",
            ".runseal/lib/cli.ts",
            ".runseal/lib/hash.ts",
            ".runseal/lib/negentropy.ts",
            ".runseal/lib/std/cmd.ts",
            ".runseal/lib/std/env.ts",
            ".runseal/lib/std/fs.ts",
            ".runseal/lib/std/io.ts",
            ".runseal/lib/std/json.ts",
            ".runseal/lib/std/path.ts",
            ".runseal/lib/std/runseal.ts",
            ".runseal/lib/version.ts",
            ".runseal/templates/cloudflare.env",
            ".runseal/wrappers/cloudflare.ts",
            ".runseal/wrappers/guard.ts",
            ".runseal/wrappers/init.ts",
            ".runseal/wrappers/land.ts",
            ".runseal/wrappers/release.ts",
            ".forgejo/release.env.example",
            ".forgejo/workflows/guard.yml",
            ".forgejo/workflows/release-beta.yml",
            ".forgejo/workflows/release-stable.yml",
            ".forgejo/scripts/release/assets/checksums.sh",
            ".forgejo/scripts/release/assets/package.sh",
            ".forgejo/scripts/release/assets/verify.sh",
            ".forgejo/scripts/release/metadata/beta.ts",
            ".forgejo/scripts/release/metadata/stable.ts",
            ".forgejo/scripts/release/r2/check.sh",
            ".forgejo/scripts/release/r2/publish.sh",
            ".forgejo/scripts/release/r2/summary.sh",
            ".forgejo/scripts/release/r2/verify.sh",
            ".forgejo/scripts/release/smoke/smoke.sh",
        ] {
            let file = project.join(path);
            std::fs::create_dir_all(file.parent().expect("file should have a parent"))
                .expect("parent should be created");
            std::fs::write(&file, "").expect("required file should be written");
        }
        std::fs::write(
            project.join(".runseal/deno.lock"),
            std::fs::read_to_string(Self::root().join(".runseal/deno.lock"))
                .expect("repo deno lock should be readable"),
        )
        .expect("deno lock should be copied");
        std::fs::write(
            project.join(".runseal/wrappers/init.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/wrappers/init.ts"))
                .expect("repo init wrapper should be readable"),
        )
        .expect("init wrapper should be copied");
        std::fs::write(
            project.join(".runseal/wrappers/guard.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/wrappers/guard.ts"))
                .expect("repo guard wrapper should be readable"),
        )
        .expect("guard wrapper should be copied");
        std::fs::write(
            project.join(".runseal/lib/cli.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/lib/cli.ts"))
                .expect("repo cli helper should be readable"),
        )
        .expect("cli helper should be copied");
        for path in [
            ".runseal/lib/std/cmd.ts",
            ".runseal/lib/std/env.ts",
            ".runseal/lib/std/fs.ts",
            ".runseal/lib/std/io.ts",
            ".runseal/lib/std/json.ts",
            ".runseal/lib/std/path.ts",
            ".runseal/lib/std/runseal.ts",
        ] {
            std::fs::write(
                project.join(path),
                std::fs::read_to_string(Self::root().join(path))
                    .expect("repo std helper should be readable"),
            )
            .expect("std helper should be copied");
        }
        std::fs::write(
            project.join(".runseal/lib/hash.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/lib/hash.ts"))
                .expect("repo hash helper should be readable"),
        )
        .expect("hash helper should be copied");
        std::fs::write(
            project.join(".runseal/negentropy.version"),
            std::fs::read_to_string(Self::root().join(".runseal/negentropy.version"))
                .expect("repo negentropy version should be readable"),
        )
        .expect("negentropy version should be copied");
        std::fs::write(
            project.join(".runseal/lib/negentropy.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/lib/negentropy.ts"))
                .expect("repo negentropy helper should be readable"),
        )
        .expect("negentropy helper should be copied");
        std::fs::write(
            project.join(".runseal/lib/version.ts"),
            std::fs::read_to_string(Self::root().join(".runseal/lib/version.ts"))
                .expect("repo version helper should be readable"),
        )
        .expect("version helper should be copied");
        std::fs::write(
            project.join(".runseal/deno.json"),
            std::fs::read_to_string(Self::root().join(".runseal/deno.json"))
                .expect("repo deno config should be readable"),
        )
        .expect("deno config should be copied");
        std::fs::write(
            project.join(".runseal/hooks/pre-commit"),
            std::fs::read_to_string(Self::root().join(".runseal/hooks/pre-commit"))
                .expect("repo pre-commit hook should be readable"),
        )
        .expect("pre-commit hook should be copied");
        std::fs::write(
            project.join(".runseal/hooks/commit-msg"),
            std::fs::read_to_string(Self::root().join(".runseal/hooks/commit-msg"))
                .expect("repo commit-msg hook should be readable"),
        )
        .expect("commit-msg hook should be copied");
        std::fs::write(
            project.join(".runseal/templates/cloudflare.env"),
            std::fs::read_to_string(Self::root().join(".runseal/templates/cloudflare.env"))
                .expect("repo cloudflare template should be readable"),
        )
        .expect("cloudflare template should be copied");
        std::fs::write(
            project.join("runseal.toml"),
            r#"
injections = []

[deno]
config = ".runseal/deno.json"
permissions = [
  "--allow-read=.",
  "--allow-write=.",
  "--allow-env",
  "--allow-run=git,deno,cargo,runseal,negentropy,sh,bash,sed,grep",
]
"#,
        )
        .expect("profile should be written");
    }
}

struct Git;

impl Git {
    fn write(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            r#"#!/bin/sh
set -eu
case "${1:-}" in
  --version)
    ;;
  rev-parse)
    if [ "${2:-}" = "--show-toplevel" ]; then
      pwd
    else
      exit 9
    fi
    ;;
  config)
    if [ "${2:-}" = "core.hooksPath" ] && [ "${3:-}" = ".runseal/hooks" ]; then
      exit 0
    fi
    if [ "${2:-}" = "--get" ] && [ "${3:-}" = "core.hooksPath" ]; then
      printf '%s\n' ".runseal/hooks"
      exit 0
    fi
    exit 9
    ;;
  *)
    exit 9
    ;;
esac
"#,
        )
        .expect("git stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("git stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("git stub should be executable");
    }
}

struct Script;

impl Script {
    fn write(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--version" ] && [ "${0##*/}" = "negentropy" ]; then
  printf '%s\n' 'negentropy v0.1.0-beta.10'
fi
if [ "${1:-}" = "config" ] && [ "${2:-}" = "--get" ]; then
  if [ "${3:-}" = "core.hooksPath" ]; then
    printf '%s\n' ".runseal/hooks"
  fi
fi
exit 0
"#,
        )
        .expect("stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("stub should be executable");
    }
}

impl Fixture {
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app dir should have repo parent")
            .to_path_buf()
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_runseal"))
            .current_dir(&self.project)
            .env("PATH", self.path())
            .arg("-p")
            .arg(self.project.join("runseal.toml"))
            .arg(":init")
            .args(args)
            .output()
            .expect("runseal init should run")
    }

    fn path(&self) -> OsString {
        let mut paths = vec![self.bin.clone()];
        if let Some(runseal) = Path::new(env!("CARGO_BIN_EXE_runseal")).parent() {
            paths.push(runseal.to_path_buf());
        }
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }
}

mod help {
    use super::*;

    #[test]
    fn reads() {
        let fx = fixture();

        let output = fx.run(&["--help"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Usage: runseal :init"));
    }
}

#[test]
fn pinned() {
    let fx = fixture();

    let output = fx.run(&[]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mismatch() {
    let fx = fixture();
    std::fs::write(
        fx.bin.join("negentropy"),
        "#!/bin/sh\nprintf '%s\\n' 'negentropy v0.1.0-beta.11'\n",
    )
    .expect("negentropy stub should be replaced");

    let output = fx.run(&[]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("negentropy: expected v0.1.0-beta.10, got negentropy v0.1.0-beta.11")
    );
}
