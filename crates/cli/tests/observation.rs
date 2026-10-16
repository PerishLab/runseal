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

const CASES: [&[&str]; 8] = [
    &["profile"],
    &["resolve", "invalid-private-uri"],
    &["unknown-control"],
    &[":"],
    &[":", "sh", "-c", "printf held; exit 0"],
    &[":", "sh", "-c", "printf held >&2; exit 17"],
    &[":absent", "true"],
    &[":", "@missing"],
];

const HELD: &str = "[env.vars]\nPRIVATE_MARKER = 'private-env-value'\n";
const REFUSED: &str = "[env.vars]\n'private key' = 'private-env-value'\n";

fn seat(profile: &str) -> tempfile::TempDir {
    let seat = tempfile::tempdir().expect("seat");
    std::fs::write(seat.path().join("runseal.toml"), profile).unwrap();
    seat
}

#[test]
fn fault() {
    let held = seat(HELD);
    let refused = seat(REFUSED);
    configure(held.path(), ENABLED);
    configure(refused.path(), ENABLED);
    let mut codes = Vec::new();
    for args in CASES {
        codes.push(run(held.path(), args).status.code());
    }
    codes.push(run(refused.path(), &[":", "true"]).status.code());
    for code in [Some(0), Some(1), Some(17)] {
        assert!(codes.contains(&code));
    }
    let mut atoms = records(held.path());
    atoms.extend(records(refused.path()));
    let finishes: Vec<Value> = atoms
        .into_iter()
        .filter(|atom| atom["payload"]["event"] == "cli.finish")
        .collect();
    assert_eq!(finishes.len(), codes.len());
    assert!(finishes.iter().any(|atom| atom["payload"]["code"] == 0));
    assert!(
        finishes
            .iter()
            .all(|atom| atom["context"].get("runseal.fault").is_none())
    );
    assert!(
        !serde_json::to_string(&finishes)
            .unwrap()
            .contains("private")
    );
}

#[test]
fn parity() {
    for args in CASES {
        compare(HELD, args);
    }
    compare(REFUSED, &[":", "true"]);
}

fn compare(profile: &str, args: &[&str]) {
    let home = seat(profile);
    let quiet = run(home.path(), args);
    assert!(!home.path().join("state/locus").exists());
    configure(home.path(), ENABLED);
    let observed = run(home.path(), args);
    assert!(home.path().join("state/locus").exists());
    assert_eq!(quiet.status.code(), observed.status.code(), "{args:?}");
    assert_eq!(quiet.stdout, observed.stdout, "{args:?}");
    assert_eq!(lines(&quiet.stderr), lines(&observed.stderr), "{args:?}");
}

fn lines(stderr: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| !line.starts_with("locus engine diagnostic"))
        .map(str::to_string)
        .collect()
}
