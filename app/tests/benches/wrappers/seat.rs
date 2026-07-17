#![cfg(unix)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

pub struct File;

impl File {
    pub fn shell(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.sh"))
    }

    pub fn ts(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.ts"))
    }

    pub fn write(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, content).expect("executable should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("executable metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("executable should be executable");
    }
}

pub struct Wrapper;

impl Wrapper {
    pub fn shell(path: &Path, label: &str) {
        File::write(path, &format!("#!/usr/bin/env sh\nprintf '{}\\n'\n", label));
    }

    pub fn ts(path: &Path) {
        std::fs::write(path, "console.log(Deno.args.join('|'));\n")
            .expect("ts wrapper should be written");
    }
}

pub struct Fixture {
    pub _temp: TempDir,
    pub project: PathBuf,
    pub home: PathBuf,
    pub bin: PathBuf,
    pub local: PathBuf,
    pub global: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should be created");
        let project = temp.path().join("project");
        let home = temp.path().join("home");
        let bin = temp.path().join("bin");
        let local = project.join(".runseal").join("wrappers");
        let global = home.join("wrappers");
        std::fs::create_dir_all(project.join(".runseal")).expect("project .runseal should exist");
        std::fs::create_dir_all(&local).expect("project wrappers should be created");
        std::fs::create_dir_all(&global).expect("home wrappers should be created");
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
        Self {
            _temp: temp,
            project,
            home,
            bin,
            local,
            global,
        }
    }

    pub fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.project)
            .env("RUNSEAL_HOME", &self.home)
            .env("PATH", Paths::prepend(&self.bin))
            .args(args)
            .output()
            .expect("runseal should run")
    }

    pub fn deno(&self) -> PathBuf {
        let log = self.project.join("deno.log");
        File::write(
            &self.bin.join("deno"),
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
}

pub struct Paths;

impl Paths {
    pub fn prepend(first: &Path) -> OsString {
        let mut paths = vec![first.to_path_buf()];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }

    pub fn suffix(path: &Path, count: usize) -> PathBuf {
        path.components()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

pub struct Assert;

impl Assert {
    pub fn path(actual: &str, expected: &Path) {
        let expected = Paths::suffix(expected, 4);
        assert!(
            Path::new(actual).ends_with(&expected),
            "expected {actual:?} to end with {}",
            expected.display()
        );
    }
}
