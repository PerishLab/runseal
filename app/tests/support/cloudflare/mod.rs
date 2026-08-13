use std::{collections::BTreeMap, fs};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::support::{Seat, sequence, serve, text};
use serde_json::{Value, json};

mod control;

struct Case {
    args: Vec<&'static str>,
    method: &'static str,
    route: String,
    result: Value,
}

#[test]
fn surface() {
    for owner in ["account", "user"] {
        for case in cases(owner) {
            check(owner, case);
        }
    }
}

#[test]
fn pages() {
    let first = (0..50).map(|id| json!({"id": id})).collect::<Vec<_>>();
    let answers = vec![
        envelope(Value::Array(first), json!({"total_pages": 2})),
        envelope(json!([{"id": 50}]), json!({"total_pages": 2})),
    ];
    let (url, handle) = sequence(
        answers
            .into_iter()
            .map(|body| ("200 OK".into(), body))
            .collect(),
    );
    let vars = vars(&url);
    let args = words(&["token", "user", "list"]);
    let reply = runseal::tool::call("cloudflare", &args, &vars).expect("token list");
    let seen = handle.join().expect("server");
    assert_eq!(reply.value.as_array().map(Vec::len), Some(51));
    assert!(seen[1].contains("page=2"));
}

#[test]
fn secret() {
    let generated = "generated-secret-value-which-is-never-printed";
    let body = envelope(
        json!({"id": "token-id", "name": "held", "value": generated}),
        Value::Null,
    );
    let (url, handle) = serve("200 OK", &body, 1);
    let seat = Seat::new();
    profile(&seat, &url);
    let body = seat.home.join("token.json");
    let value = seat.home.join("generated.token");
    fs::write(&body, r#"{"name":"held","policies":[]}"#).expect("body");
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--json",
        "--body-file",
        body.to_str().expect("body path"),
        "--value-file",
        value.to_str().expect("value path"),
        "token",
        "account",
        "create",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert!(!stdout.contains(generated));
    assert!(!stderr.contains(generated));
    assert!(stdout.contains("token-id"));
    assert!(stdout.contains("value_sha256"));
    assert_eq!(
        fs::read_to_string(&value).expect("secret"),
        format!("{generated}\n")
    );
    #[cfg(unix)]
    {
        let mode = fs::metadata(&value).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn guarded() {
    let generated = "generated-secret-value-which-is-never-debugged";
    let body = envelope(json!({"id": "token-id", "value": generated}), Value::Null);
    let (url, handle) = serve("200 OK", &body, 1);
    let vars = vars(&url);
    let args = words(&["token", "user", "create"]);
    let input = json!({"name": "held", "policies": []});
    let reply = runseal::tool::cloudflare::invoke(&args, &vars, Some(input)).expect("create");
    handle.join().expect("server");
    assert_eq!(reply.secret().map(|value| value.expose()), Some(generated));
    assert!(!format!("{reply:?}").contains(generated));
    assert!(reply.value.get("value").is_none());
}

#[test]
fn occupied() {
    let seat = Seat::new();
    profile(&seat, "http://127.0.0.1:1");
    let body = seat.home.join("token.json");
    let value = seat.home.join("existing.token");
    fs::write(&body, r#"{"name":"held","policies":[]}"#).expect("body");
    fs::write(&value, "keep").expect("existing");
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--body-file",
        body.to_str().expect("body path"),
        "--value-file",
        value.to_str().expect("value path"),
        "token",
        "user",
        "create",
    ]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unable to reserve token value file"));
    assert_eq!(fs::read_to_string(value).expect("existing"), "keep");
}

#[test]
fn refusal() {
    let body = json!({
        "success": false,
        "errors": [{"code": 9109, "message": "unauthorized"}],
        "messages": [],
        "result": null
    })
    .to_string();
    let (url, handle) = serve("403 Forbidden", &body, 1);
    let error = runseal::tool::call(
        "cloudflare",
        &words(&["token", "user", "verify"]),
        &vars(&url),
    )
    .expect_err("refusal");
    handle.join().expect("server");
    let fault = error
        .downcast_ref::<runseal::tool::cloudflare::api::Fault>()
        .expect("cloudflare fault");
    assert_eq!(fault.status(), 403);
    assert_eq!(fault.codes(), &[9109]);
    assert_eq!(error.to_string(), "cloudflare refused (403): unauthorized");
}

fn cases(owner: &str) -> Vec<Case> {
    let stem = if owner == "account" {
        "/client/v4/accounts/account-id/tokens"
    } else {
        "/client/v4/user/tokens"
    };
    vec![
        Case {
            args: vec!["list"],
            method: "GET",
            route: format!("{stem}?page=1&per_page=50"),
            result: json!([]),
        },
        Case {
            args: vec!["show", "token-id"],
            method: "GET",
            route: format!("{stem}/token-id"),
            result: json!({"id": "token-id"}),
        },
        Case {
            args: vec!["create"],
            method: "POST",
            route: stem.to_string(),
            result: json!({"id": "token-id", "value": "generated-secret"}),
        },
        Case {
            args: vec!["edit", "token-id"],
            method: "PUT",
            route: format!("{stem}/token-id"),
            result: json!({"id": "token-id"}),
        },
        Case {
            args: vec!["roll", "token-id"],
            method: "PUT",
            route: format!("{stem}/token-id/value"),
            result: json!({"value": "generated-secret"}),
        },
        Case {
            args: vec!["delete", "token-id"],
            method: "DELETE",
            route: format!("{stem}/token-id"),
            result: json!({"id": "token-id"}),
        },
        Case {
            args: vec!["verify"],
            method: "GET",
            route: format!("{stem}/verify"),
            result: json!({"id": "token-id", "status": "active"}),
        },
        Case {
            args: vec!["permission", "list"],
            method: "GET",
            route: format!("{stem}/permission_groups"),
            result: json!([]),
        },
    ]
}

fn check(owner: &str, case: Case) {
    let body = envelope(case.result, json!({"total_pages": 1}));
    let (url, handle) = serve("200 OK", &body, 1);
    let temp = tempfile::tempdir().expect("temp");
    let body = temp.path().join("token.json");
    fs::write(&body, r#"{"name":"held","policies":[]}"#).expect("body");
    let mut args = vec![
        "--body-file",
        body.to_str().expect("body path"),
        "token",
        owner,
    ];
    args.extend(case.args);
    runseal::tool::call("cloudflare", &words(&args), &vars(&url)).expect("operation");
    let seen = handle.join().expect("server");
    let expected = format!("{} {} ", case.method, case.route);
    assert!(seen[0].starts_with(&expected), "{}", seen[0]);
}

fn vars(url: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("CLOUDFLARE_API_URL".into(), format!("{url}/client/v4")),
        ("CLOUDFLARE_ACCOUNT_ID".into(), "account-id".into()),
        ("CLOUDFLARE_API_TOKEN".into(), "factory-token".into()),
    ])
}

fn profile(seat: &Seat, url: &str) {
    let body = format!(
        "[env.vars]\nCLOUDFLARE_API_URL = \"{url}/client/v4\"\nCLOUDFLARE_ACCOUNT_ID = \"account-id\"\nCLOUDFLARE_API_TOKEN = \"factory-token\"\n"
    );
    fs::write(seat.home.join("profiles/perish/runseal.toml"), body).expect("profile");
}

fn envelope(result: Value, info: Value) -> String {
    json!({
        "success": true,
        "errors": [],
        "messages": [],
        "result": result,
        "result_info": info
    })
    .to_string()
}

fn words(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}
