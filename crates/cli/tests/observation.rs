use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
        .current_dir(home)
        .env("RUNSEAL_HOME", home)
        .args(args)
        .output()
        .expect("command")
}

fn configure(home: &Path, text: &str) {
    std::fs::write(home.join("config.toml"), text).expect("configuration");
}

const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

fn records(home: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(home.join("state/locus")).expect("buffer") {
        let path = entry.expect("entry").path();
        let name = path.file_name().expect("name").to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            let content = std::fs::read_to_string(path).expect("records");
            records.extend(
                content
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).expect("atom")),
            );
        }
    }
    records
}

#[test]
fn disabled() {
    let home = tempfile::tempdir().expect("home");
    let absent = run(home.path(), &["profile"]);
    configure(home.path(), "[locus]\nenabled = false\n");
    let disabled = run(home.path(), &["profile"]);
    assert_eq!(absent.status.code(), disabled.status.code());
    assert_eq!(absent.stdout, disabled.stdout);
    assert_eq!(absent.stderr, disabled.stderr);
    assert!(!home.path().join("state/locus").exists());
}

#[test]
fn control() {
    let home = tempfile::tempdir().expect("home");
    configure(home.path(), ENABLED);
    for args in [vec!["profile"], vec!["resolve", "invalid-private-uri"]] {
        run(home.path(), &args);
    }
    let atoms = records(home.path());
    assert!(atoms.iter().all(|atom| atom["producer"] == "runseal"));
    for code in [0, 1] {
        assert!(atoms.iter().any(
            |atom| atom["payload"]["event"] == "cli.finish" && atom["payload"]["code"] == code
        ));
    }
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            4
        );
    }
    assert!(
        !serde_json::to_string(&atoms)
            .unwrap()
            .contains("invalid-private-uri")
    );
}

#[test]
fn isolated() {
    let home = tempfile::tempdir().expect("home");
    let baseline = run(home.path(), &["profile"]);
    for config in [
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        "[locus]\nenabled = 'malformed'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        configure(home.path(), config);
        let result = run(home.path(), &["profile"]);
        assert_eq!(result.status.code(), baseline.status.code());
        assert_eq!(result.stdout, baseline.stdout);
        assert!(!result.stderr.is_empty(), "operator diagnostic");
    }
    let blocked = tempfile::tempdir().expect("home");
    std::fs::write(blocked.path().join("state"), "occupied").unwrap();
    configure(blocked.path(), ENABLED);
    let result = run(blocked.path(), &["profile"]);
    assert!(result.status.success());
    assert!(!result.stderr.is_empty());
    assert_eq!(
        std::fs::read_to_string(blocked.path().join("state")).unwrap(),
        "occupied"
    );
}

#[test]
#[cfg(unix)]
fn projection() {
    let home = tempfile::tempdir().expect("home");
    let root = home.path();
    std::fs::create_dir_all(root.join(".runseal/resources")).unwrap();
    std::fs::write(root.join(".runseal/resources/source"), "held").unwrap();
    std::fs::write(
        root.join("runseal.toml"),
        "[env.vars]\nPRIVATE_MARKER = 'private-env-value'\n[argv]\nbash = ['--noprofile']\n[[symlink]]\nsource = 'resource://source'\ntarget = 'local://seat'\n",
    ).unwrap();
    let args = [
        ":",
        "bash",
        "-c",
        "test -L .local/seat && printf '%s' \"$PRIVATE_MARKER\"; exit 17",
    ];
    let baseline = run(root, &args);
    configure(root, ENABLED);
    let observed = run(root, &args);
    assert_eq!(baseline.status.code(), Some(17));
    assert_eq!(observed.status.code(), baseline.status.code());
    assert_eq!(observed.stdout, baseline.stdout);
    assert_eq!(observed.stdout, b"private-env-value");
    assert!(!root.join(".local/seat").exists());
    let atoms = records(root);
    assert!(
        atoms
            .iter()
            .all(|atom| atom["context"]["runseal.command"] == "profile.run")
    );
    assert!(atoms.iter().any(|atom| atom["payload"]["code"] == 17));
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("private-env-value"));
    assert!(!text.contains("PRIVATE_MARKER"));
    assert!(!text.contains("test -L"));
}

#[test]
fn tool() {
    let home = tempfile::tempdir().expect("home");
    configure(home.path(), ENABLED);
    let result = run(home.path(), &[":", "@missing"]);
    assert_eq!(result.status.code(), Some(1));
    let atoms = records(home.path());
    assert!(
        atoms
            .iter()
            .all(|atom| atom["context"]["runseal.command"] == "tool")
    );
    assert!(atoms.iter().any(|atom| atom["payload"]["code"] == 1));
    assert!(!serde_json::to_string(&atoms).unwrap().contains("@missing"));
}
