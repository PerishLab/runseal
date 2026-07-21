#![cfg(unix)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

use super::stub::{Git, Script, root};

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
    for tool in ["cargo", "negentropy", "sh", "bash", "sed", "grep"] {
        Script::write(&bin.join(tool));
    }
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
            ".runseal/hooks/pre-commit",
            ".runseal/hooks/commit-msg",
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
            std::fs::read_to_string(root().join(".runseal/deno.lock"))
                .expect("repo deno lock should be readable"),
        )
        .expect("deno lock should be copied");
        std::fs::write(
            project.join(".runseal/wrappers/init.ts"),
            std::fs::read_to_string(root().join(".runseal/wrappers/init.ts"))
                .expect("repo init wrapper should be readable"),
        )
        .expect("init wrapper should be copied");
        std::fs::write(
            project.join(".runseal/wrappers/guard.ts"),
            std::fs::read_to_string(root().join(".runseal/wrappers/guard.ts"))
                .expect("repo guard wrapper should be readable"),
        )
        .expect("guard wrapper should be copied");
        std::fs::write(
            project.join(".runseal/deno.json"),
            std::fs::read_to_string(root().join(".runseal/deno.json"))
                .expect("repo deno config should be readable"),
        )
        .expect("deno config should be copied");
        std::fs::write(
            project.join(".runseal/hooks/pre-commit"),
            std::fs::read_to_string(root().join(".runseal/hooks/pre-commit"))
                .expect("repo pre-commit hook should be readable"),
        )
        .expect("pre-commit hook should be copied");
        std::fs::write(
            project.join(".runseal/hooks/commit-msg"),
            std::fs::read_to_string(root().join(".runseal/hooks/commit-msg"))
                .expect("repo commit-msg hook should be readable"),
        )
        .expect("commit-msg hook should be copied");
        std::fs::write(
            project.join(".runseal/templates/cloudflare.env"),
            std::fs::read_to_string(root().join(".runseal/templates/cloudflare.env"))
                .expect("repo cloudflare template should be readable"),
        )
        .expect("cloudflare template should be copied");
        std::fs::write(
            project.join("runseal.toml"),
            r#"
injections = []

[deno]
config = ".runseal/deno.json"
lock = ".runseal/deno.lock"
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

impl Fixture {
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
