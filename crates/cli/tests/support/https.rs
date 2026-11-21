use super::{configure, run};

#[test]
fn https() {
    let seat = tempfile::tempdir().unwrap();
    let home = seat.path();
    let key = home.join("writer");
    std::fs::write(&key, "fixture\n").unwrap();
    configure(
        home,
        &format!(
            "[locus]\nenabled=true\nendpoint='https://127.0.0.1:0'\nkey_file={}\n",
            serde_json::to_string(&key).unwrap()
        ),
    );
    let observed = run(home, &["profile"]);
    assert!(observed.status.success());
    assert!(observed.stderr.is_empty());
    let queue = home.join("state/locus.https/runseal");
    let binding: serde_json::Value =
        serde_json::from_slice(&std::fs::read(queue.join("delivery.json")).unwrap()).unwrap();
    assert_eq!(binding["producer"], "runseal");
    assert_eq!(binding["format"], "sqlite-wal/v1");
    assert!(queue.join("queue.db").is_file());
    assert!(!home.join("state/locus").exists());
}
