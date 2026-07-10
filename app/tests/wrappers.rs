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

struct File;

impl File {
    fn shell(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.sh"))
    }

    fn ts(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.ts"))
    }

    fn write(path: &Path, content: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, content).expect("executable should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("executable metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("executable should be executable");
    }
}

struct Wrapper;

impl Wrapper {
    fn shell(path: &Path, label: &str) {
        File::write(path, &format!("#!/usr/bin/env sh\nprintf '{}\\n'\n", label));
    }

    fn ts(path: &Path) {
        std::fs::write(path, "console.log(Deno.args.join('|'));\n")
            .expect("ts wrapper should be written");
    }
}

struct Fixture {
    _temp: TempDir,
    project: PathBuf,
    home: PathBuf,
    bin: PathBuf,
    local: PathBuf,
    global: PathBuf,
}

impl Fixture {
    fn new() -> Self {
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

    fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.project)
            .env("RUNSEAL_HOME", &self.home)
            .env("PATH", Paths::prepend(&self.bin))
            .args(args)
            .output()
            .expect("runseal should run")
    }

    fn deno(&self) -> PathBuf {
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

struct Paths;

impl Paths {
    fn prepend(first: &Path) -> OsString {
        let mut paths = vec![first.to_path_buf()];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("PATH should be joinable")
    }

    fn suffix(path: &Path, count: usize) -> PathBuf {
        path.components()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

struct Assert;

impl Assert {
    fn path(actual: &str, expected: &Path) {
        let expected = Paths::suffix(expected, 4);
        assert!(
            Path::new(actual).ends_with(&expected),
            "expected {actual:?} to end with {}",
            expected.display()
        );
    }
}

#[test]
fn visible() {
    let fx = Fixture::new();
    Wrapper::ts(&File::ts(&fx.local, "wrap"));
    Wrapper::shell(&File::shell(&fx.global, "wrap"), "home");
    Wrapper::ts(&File::ts(&fx.global, "home-only"));

    let output = fx.run(&["@wrappers"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains(":wrap"));
    assert!(stdout.contains(":home-only"));
    assert!(stdout.contains("profile"));
    assert!(stdout.contains("home"));
    let line = stdout
        .lines()
        .find(|line| line.contains(":wrap"))
        .expect("wrap should be listed");
    let file = line
        .split_whitespace()
        .last()
        .expect("wrap line should include a file");
    assert!(line.contains("profile"));
    assert!(
        Path::new(file).ends_with(Paths::suffix(&File::ts(&fx.local, "wrap"), 4)),
        "expected {file} to point at the profile wrapper"
    );
}

#[test]
fn resolve() {
    let fx = Fixture::new();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);

    let which = fx.run(&["@which", ":tool"]);

    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &wrapper);
}

#[test]
fn extension() {
    let fx = Fixture::new();
    Wrapper::shell(&fx.local.join("legacy"), "legacy");

    let output = fx.run(&[":legacy"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("wrapper not found: :legacy"));
    assert!(stderr.contains("legacy.ts"));
    assert!(stderr.contains("legacy.sh"));
    assert!(!stderr.contains(".runseal/wrappers/legacy\n"));
}

#[test]
fn policy() {
    let fx = Fixture::new();
    let log = fx.deno();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", Paths::prepend(&fx.bin))
        .env("RUNSEAL_TEST_DENO_LOG", &log)
        .args([":tool", "hello", "world"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("name=tool"));
    Assert::path(
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
fn expand() {
    let fx = Fixture::new();
    let log = fx.deno();
    let wrapper = File::ts(&fx.local, "tool");
    Wrapper::ts(&wrapper);
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
        .env("PATH", Paths::prepend(&fx.bin))
        .args([":tool"])
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let log = std::fs::read_to_string(log).expect("deno log should be readable");
    assert!(log.contains(&format!("--allow-read={}", allowed.display())));
    assert!(!log.contains("${RUNSEAL_TEST_READ}"));
}

#[test]
fn required() {
    let fx = Fixture::new();
    std::fs::write(
        fx.project.join("runseal.toml"),
        "injections = []\n[resources]\nroot = \".resource\"\n",
    )
    .expect("profile should be written");
    Wrapper::ts(&File::ts(&fx.local, "tool"));

    let output = fx.run(&[":tool"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("deno wrapper requires a [deno] profile policy"));
}

#[test]
fn precedence() {
    let fx = Fixture::new();
    let log = fx.deno();
    Wrapper::ts(&File::ts(&fx.local, "tool"));
    Wrapper::shell(&File::shell(&fx.local, "tool"), "shell");

    let output = bin()
        .current_dir(&fx.project)
        .env("RUNSEAL_HOME", &fx.home)
        .env("PATH", Paths::prepend(&fx.bin))
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
fn shell() {
    let fx = Fixture::new();
    Wrapper::shell(&File::shell(&fx.local, "tool"), "shell");

    let output = fx.run(&[":tool"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "shell\n");
}

#[test]
fn shadow() {
    let fx = Fixture::new();
    let wrapper = File::ts(&fx.local, "wrap");
    Wrapper::ts(&wrapper);
    Wrapper::shell(&File::shell(&fx.global, "wrap"), "home");

    let which = fx.run(&["@which", ":wrap"]);
    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &wrapper);

    let output = fx.run(&["@wrappers"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let lines = stdout
        .lines()
        .filter(|line| line.starts_with(":wrap "))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("profile"));
}
