#[path = "tools/forgejo.rs"]
#[cfg(unix)]
mod forgejo;
#[path = "tools/github.rs"]
#[cfg(unix)]
mod github;

use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

#[test]
fn progressive() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).expect("empty cwd should be created");

    for (args, expected) in [
        (
            vec!["@tool", "github", "issue", "comment", "create", "--help"],
            "--prefix-enable=<true|false>",
        ),
        (
            vec!["@tool", "cloudflare", "config", "--help"],
            "Cloudflare config helpers:",
        ),
        (
            vec!["@tool", "cloudflare", "zone", "dns-record", "--help"],
            "Usage: runseal @tool cloudflare zone dns-record <command> [args]",
        ),
        (
            vec![
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "list",
                "--help",
            ],
            "--name <name>",
        ),
        (
            vec![
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "create",
                "--help",
            ],
            "Create one DNS record",
        ),
        (
            vec!["@tool", "github", "pr", "checks", "probe", "--help"],
            "on API probe failure",
        ),
        (
            vec!["@tool", "forgejo", "pr", "guard", "--help"],
            "fail closed",
        ),
        (
            vec!["@tool", "forgejo", "secret", "upsert", "--help"],
            "not exposed in process arguments",
        ),
    ] {
        let output = bin()
            .current_dir(&cwd)
            .env("RUNSEAL_HOME", temp.path().join("home"))
            .args(args.clone())
            .output()
            .expect("runseal should run");

        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert!(
            stdout.contains(expected),
            "{args:?} should contain {expected:?}, got {stdout:?}"
        );
    }
}

#[test]
fn rich() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).expect("empty cwd should be created");

    for (args, expected) in [
        (
            vec!["@tool", "cloudflare", "zone", "--help"],
            "Cloudflare zone helpers:",
        ),
        (
            vec!["@tool", "cloudflare", "zone", "ruleset", "--help"],
            "Cloudflare zone ruleset helpers:",
        ),
        (
            vec![
                "@tool",
                "cloudflare",
                "zone",
                "dns-record",
                "update",
                "--help",
            ],
            "--record-id <id> --json <json>",
        ),
        (
            vec!["@tool", "cloudflare", "redirect-rule", "exact", "--help"],
            "Build one exact-match redirect rule payload as JSON.",
        ),
        (
            vec!["@tool", "github", "issue", "comment", "create", "--help"],
            "--prefix-enable=<true|false>",
        ),
    ] {
        let output = bin()
            .current_dir(&cwd)
            .env("RUNSEAL_HOME", temp.path().join("home"))
            .args(args.clone())
            .output()
            .expect("runseal should run");

        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert!(
            stdout.contains(expected),
            "{args:?} should contain {expected:?}, got {stdout:?}"
        );
    }
}

#[test]
fn hidden() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).expect("empty cwd should be created");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .args(["@tool", "json", "pretty", "value", r#"{"a":1}"#])
        .output()
        .expect("runseal should run");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown tool command: json pretty"));
}
