use std::fs;

use serde_json::{Value, json};

use super::{envelope, profile, vars, words};
use crate::support::{Seat, serve, text};

#[test]
fn written() {
    let generated = "rolled-secret-value-which-is-never-printed";
    let body = envelope(json!(generated), Value::Null);
    let (url, handle) = serve("200 OK", &body, 1);
    let seat = Seat::new();
    profile(&seat, &url);
    let value = seat.home.join("rolled.token");
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--json",
        "--value-file",
        value.to_str().expect("value path"),
        "token",
        "account",
        "roll",
        "token-id",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(!stdout.contains(generated));
    assert!(!text(&output.stderr).contains(generated));
    assert!(stdout.contains("token-id"));
    assert!(stdout.contains("value_sha256"));
    assert_eq!(
        fs::read_to_string(&value).expect("secret"),
        format!("{generated}\n")
    );
}

#[test]
fn dead() {
    let body = envelope(Value::Null, Value::Null);
    let (url, handle) = serve("200 OK", &body, 1);
    let seat = Seat::new();
    profile(&seat, &url);
    let value = seat.home.join("lost.token");
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--value-file",
        value.to_str().expect("value path"),
        "token",
        "account",
        "roll",
        "token-id",
    ]);
    handle.join().expect("server");
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("previous value is already dead"),
        "{stderr}"
    );
    assert!(stderr.contains("roll it again at once"), "{stderr}");
    assert!(!value.exists());
}

#[test]
fn crossed() {
    let body = json!({
        "success": false,
        "errors": [{"code": 1000, "message": "Invalid API Token"}],
        "messages": [],
        "result": null
    })
    .to_string();
    let (url, handle) = serve("401 Unauthorized", &body, 1);
    let error = runseal::tool::call(
        "cloudflare",
        &words(&["token", "user", "verify"]),
        &vars(&url),
    )
    .expect_err("refusal");
    handle.join().expect("server");
    let report = format!("{error:#}");
    assert!(report.contains("token account verify"), "{report}");
}
