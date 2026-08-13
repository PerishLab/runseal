use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use serde_json::json;

use super::{envelope, vars, words};
use crate::support::{sequence, serve};

struct Case {
    args: &'static [&'static str],
    method: &'static str,
    route: &'static str,
}

#[test]
fn surface() {
    let cases = [
        Case {
            args: &["worker", "service", "show", "site"],
            method: "GET",
            route: "/client/v4/accounts/account-id/workers/services/site",
        },
        Case {
            args: &["worker", "domain", "list"],
            method: "GET",
            route: "/client/v4/accounts/account-id/workers/domains",
        },
        Case {
            args: &["r2", "bucket", "show", "archive"],
            method: "GET",
            route: "/client/v4/accounts/account-id/r2/buckets/archive",
        },
        Case {
            args: &["r2", "bucket", "domain", "list", "archive"],
            method: "GET",
            route: "/client/v4/accounts/account-id/r2/buckets/archive/domains/custom",
        },
        Case {
            args: &[
                "r2",
                "bucket",
                "domain",
                "delete",
                "archive",
                "assets.example.com",
            ],
            method: "DELETE",
            route: "/client/v4/accounts/account-id/r2/buckets/archive/domains/custom/assets.example.com",
        },
        Case {
            args: &["r2", "bucket", "delete", "archive"],
            method: "DELETE",
            route: "/client/v4/accounts/account-id/r2/buckets/archive",
        },
    ];
    for case in cases {
        check(case);
    }
}

#[test]
fn missing() {
    let mut vars = vars("http://127.0.0.1:1");
    vars.remove("CLOUDFLARE_ACCOUNT_ID");
    let error = runseal::tool::call("cloudflare", &words(&["worker", "domain", "list"]), &vars)
        .expect_err("missing account");
    assert!(error.to_string().contains("account operations require"));
}

#[test]
fn key() {
    let (url, handle) = gate();
    let vars = BTreeMap::from([
        ("CLOUDFLARE_API_URL".into(), format!("{url}/client/v4")),
        ("CLOUDFLARE_ACCOUNT_ID".into(), "account-id".into()),
        ("CLOUDFLARE_API_KEY".into(), "global-key".into()),
        ("CLOUDFLARE_API_EMAIL".into(), "owner@example.com".into()),
    ]);
    runseal::tool::call("cloudflare", &words(&["worker", "domain", "list"]), &vars)
        .expect("key auth");
    assert!(handle.join().expect("server"));
}

#[test]
fn retry() {
    let refused = json!({
        "success": false,
        "errors": [{"code": 10000, "message": "busy"}],
        "messages": [],
        "result": null
    })
    .to_string();
    let accepted = envelope(json!([]), serde_json::Value::Null);
    let (url, handle) = sequence(vec![
        ("429 Too Many Requests".into(), refused),
        ("200 OK".into(), accepted),
    ]);
    runseal::tool::call(
        "cloudflare",
        &words(&["worker", "domain", "list"]),
        &vars(&url),
    )
    .expect("retried read");
    assert_eq!(handle.join().expect("server").len(), 2);
}

fn check(case: Case) {
    let body = envelope(json!({}), serde_json::Value::Null);
    let (url, handle) = serve("200 OK", &body, 1);
    runseal::tool::call("cloudflare", &words(case.args), &vars(&url)).expect("operation");
    let seen = handle.join().expect("server");
    let expected = format!("{} {} ", case.method, case.route);
    assert!(seen[0].starts_with(&expected), "{}", seen[0]);
}

fn gate() -> (String, thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut bytes = [0u8; 4096];
        let count = stream.read(&mut bytes).expect("request");
        let request = String::from_utf8_lossy(&bytes[..count]).to_ascii_lowercase();
        let body = envelope(json!([]), serde_json::Value::Null);
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).expect("reply");
        request.contains("x-auth-key: global-key")
            && request.contains("x-auth-email: owner@example.com")
    });
    (format!("http://{addr}"), handle)
}
