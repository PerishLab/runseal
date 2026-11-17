use super::support::{Seat, text};

#[test]
fn help() {
    let seat = Seat::new();
    for operation in [
        "begin", "complete", "refresh", "renew", "recover", "status", "logout",
    ] {
        let output = seat.run(&[":", "@alibaba", "auth", operation, "--help"]);
        assert!(output.status.success(), "{output:?}");
        assert!(text(&output.stdout).contains("--state"));
    }
    let output = seat.run(&[":", "@alibaba", "identity", "--help"]);
    assert!(output.status.success(), "{output:?}");
    for flag in ["--account", "--principal"] {
        assert!(text(&output.stdout).contains(flag));
    }
}

#[test]
fn refusal() {
    let seat = Seat::new();
    for words in [
        vec![":", "@alibaba", "auth", "status"],
        vec![
            ":",
            "@alibaba",
            "auth",
            "complete",
            "--code",
            "WITHHELD_FIXTURE",
        ],
        vec![
            ":",
            "@alibaba",
            "--endpoint",
            "https://invalid.example",
            "identity",
        ],
        vec![
            ":",
            "@alibaba",
            "--raw-body",
            "WITHHELD_FIXTURE",
            "identity",
        ],
    ] {
        let output = seat.run(&words);
        assert!(!output.status.success(), "{output:?}");
        assert!(!text(&output.stderr).contains("WITHHELD_FIXTURE"));
    }
}

#[test]
fn ceremony() {
    let seat = Seat::new();
    let path = seat.home.join("alibaba.json");
    let state = path.to_str().expect("fixture path");
    let begin = seat.run(&[":", "@alibaba", "--state", state, "--json", "auth", "begin"]);
    assert!(begin.status.success(), "{begin:?}");
    let receipt: serde_json::Value = serde_json::from_slice(&begin.stdout).expect("public receipt");
    let url = receipt["alibaba"]["authorizationUrl"]
        .as_str()
        .expect("authorization URL");
    assert!(url.starts_with("https://signin.aliyun.com/oauth2/v1/auth?"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(!text(&begin.stdout).contains("code_verifier"));
    let status = seat.run(&[
        ":", "@alibaba", "--state", state, "--json", "auth", "status",
    ]);
    assert!(status.status.success(), "{status:?}");
    let receipt: serde_json::Value = serde_json::from_slice(&status.stdout).expect("safe status");
    assert_eq!(receipt["alibaba"]["authorized"], false);
    assert_eq!(receipt["alibaba"]["credentialPresent"], false);
    assert!(receipt["alibaba"]["identity"].is_null());
    let renew = seat.run(&[":", "@alibaba", "--state", state, "auth", "renew"]);
    assert!(!renew.status.success(), "{renew:?}");
    let observed = seat.run(&[
        ":",
        "@alibaba",
        "--state",
        state,
        "--account",
        "1234567890",
        "identity",
    ]);
    assert!(!observed.status.success(), "{observed:?}");
    assert!(!text(&observed.stderr).contains("principal"));
    let relative = seat.run(&[":", "@alibaba", "--state", "relative.json", "auth", "begin"]);
    assert!(!relative.status.success(), "{relative:?}");
}
