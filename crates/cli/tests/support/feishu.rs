use super::support::{Seat, text};

#[test]
fn help() {
    let seat = Seat::new();
    for command in [
        vec![":", "@feishu", "--help"],
        vec![":", "@feishu", "auth", "renew", "--help"],
        vec![":", "@feishu", "im", "messages", "reply", "--help"],
    ] {
        let output = seat.run(&command);
        assert!(output.status.success(), "{output:?}");
        assert!(text(&output.stdout).contains("--app"));
        assert!(text(&output.stdout).contains("--state"));
    }
    let output = seat.run(&[":", "@feishu", "im", "messages", "reply", "--help"]);
    for flag in [
        "--text-file",
        "--message-id",
        "--idempotency-key",
        "--reply-in-thread",
    ] {
        assert!(text(&output.stdout).contains(flag));
    }
}

#[test]
fn refusal() {
    let seat = Seat::new();
    for words in [
        vec![":", "@feishu", "--as", "user", "auth", "status"],
        vec![
            ":",
            "@feishu",
            "--endpoint",
            "https://invalid.example",
            "auth",
            "renew",
        ],
        vec![
            ":",
            "@feishu",
            "--app-secret",
            "SECRET_FIXTURE",
            "auth",
            "renew",
        ],
        vec![
            ":",
            "@feishu",
            "im",
            "messages",
            "reply",
            "--message-id",
            "om_fixture",
        ],
    ] {
        let output = seat.run(&words);
        assert!(!output.status.success());
        assert!(!text(&output.stderr).contains("SECRET_FIXTURE"));
        assert!(!text(&output.stdout).contains("SECRET_FIXTURE"));
    }
    let output = seat.run(&[":", "@feishu", "--json", "auth", "status"]);
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["feishu"]["error"]["outcome"], "unsent");
}
