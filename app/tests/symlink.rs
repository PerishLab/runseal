#![cfg(unix)]

use std::process::Command;

use tempfile::TempDir;

fn run(root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
        .current_dir(root)
        .args([":", "bash", "-lc", "test -L .local/seat"])
        .output()
        .expect("Runseal should execute")
}

fn fixture() -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().expect("temp dir should exist");
    let root = temp.path().join("project");
    std::fs::create_dir_all(root.join(".runseal/resources")).expect("resource dir should exist");
    std::fs::write(root.join(".runseal/resources/source"), "held")
        .expect("source should be written");
    std::fs::write(
        root.join("runseal.toml"),
        r#"
[[symlink]]
source = "resource://source"
target = "local://seat"
"#,
    )
    .expect("profile should be written");
    (temp, root)
}

#[test]
fn lease() {
    let (_temp, root) = fixture();
    let output = run(&root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.join(".local/seat").exists());
}

#[test]
fn occupied() {
    let (_temp, root) = fixture();
    std::fs::create_dir_all(root.join(".local")).expect("local dir should exist");
    std::fs::write(root.join(".local/seat"), "mine").expect("target should be written");
    let output = run(&root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing occupied symlink target"));
    assert_eq!(
        std::fs::read_to_string(root.join(".local/seat")).expect("target should remain"),
        "mine"
    );
}
