use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new(profile: &str) -> Self {
        let temp = TempDir::new().expect("temp dir should exist");
        let root = temp.path().join("project");
        std::fs::create_dir_all(&root).expect("project should exist");
        std::fs::write(root.join("runseal.toml"), profile).expect("profile should be written");
        Self { _temp: temp, root }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.root)
            .args(args)
            .output()
            .expect("Runseal should execute")
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output should be UTF-8")
}

#[test]
fn help() {
    let output = bin().arg("--help").output().expect("help should run");
    assert!(output.status.success());
    let stdout = text(&output.stdout);
    assert!(stdout.contains("runseal <internal-command>"));
    assert!(stdout.contains("runseal : <command>"));
    assert!(stdout.contains("env, argv, and symlink"));
    assert!(!stdout.contains(":<wrapper>"));
    assert!(!stdout.contains(".runseal/wrappers"));
    assert!(!stdout.contains("Deno"));
}

#[test]
fn cookbook() {
    let output = bin()
        .args(["cookbook", "--help"])
        .output()
        .expect("cookbook help should run");
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("Explain recovery"));

    let output = bin().arg("cookbook").output().expect("cookbook should run");
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("no recovery entry"));
}

#[test]
fn control() {
    let output = bin()
        .arg("cargo")
        .output()
        .expect("invalid control command should run");
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("unrecognized subcommand 'cargo'"));
    assert!(!stderr.contains("profile"));
}

#[test]
fn empty() {
    let output = bin().arg(":release").output().expect("Runseal should run");
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("profile mode requires a command or @tool"));
}

#[test]
fn tool() {
    let fx = Fixture::new("");
    let output = fx.run(&[":", "@missing"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unknown Runseal tool: @missing"));
}

#[test]
#[cfg(unix)]
fn default() {
    let fx = Fixture::new(
        r#"
[env.vars]
PICKED = "profile"

[argv]
bash = ["--noprofile"]
"#,
    );
    let output = fx.run(&[
        ":",
        "bash",
        "-lc",
        "printf '%s|%s' \"$PICKED\" \"$RUNSEAL_PROFILE\"",
    ]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "profile|default");
}

#[test]
#[cfg(unix)]
fn exit() {
    let fx = Fixture::new("");
    let output = fx.run(&[":", "bash", "-lc", "exit 17"]);
    assert_eq!(output.status.code(), Some(17));
}

#[test]
#[cfg(unix)]
fn named() {
    let fx = Fixture::new("");
    std::fs::write(
        fx.root.join("runseal.release.toml"),
        "[env.vars]\nPICKED = \"release\"\n",
    )
    .expect("named profile should be written");
    let output = fx.run(&[
        ":release",
        "bash",
        "-lc",
        "printf '%s|%s' \"$PICKED\" \"$RUNSEAL_PROFILE\"",
    ]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "release|release");
}

#[test]
fn inspect() {
    let fx = Fixture::new("[env.vars]\nPICKED = \"profile\"\n");
    let output = fx.run(&["profile"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("RUNSEAL_PROFILE=default"));
    assert!(stdout.contains("RUNSEAL_ENV=1"));
    assert!(stdout.contains("RUNSEAL_ARGV=0"));
    assert!(stdout.contains("RUNSEAL_SYMLINK=0"));
    assert!(!stdout.contains("PICKED=profile"));
}

#[test]
fn resolve() {
    let fx = Fixture::new("");
    let output = fx.run(&[
        "resolve",
        "resource://cloudflare.env",
        "local://secrets/token",
    ]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    let paths = stdout.lines().map(Path::new).collect::<Vec<_>>();
    assert!(paths[0].ends_with(".runseal/resources/cloudflare.env"));
    assert!(paths[1].ends_with(".local/secrets/token"));
}
