#[path = "support/cloudflare/mod.rs"]
mod cloudflare;
mod support;

use std::collections::BTreeMap;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use support::{Seat, serve, text};

#[test]
fn absent() {
    let seat = Seat::new();
    std::fs::write(seat.home.join("profiles/perish/runseal.toml"), "").expect("empty");
    let output = seat.run(&[":perish", "@forgejo", "user", "show"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("FORGEJO_URL"));
}

#[test]
fn user() {
    let (url, handle) = serve("200 OK", r#"{"login":"perish"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "--json", "user", "show"]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains(r#""version":1"#) || stdout.contains(r#""version": 1"#));
    assert!(stdout.contains(r#""login":"perish""#) || stdout.contains(r#""login": "perish""#));
}

#[test]
fn issue() {
    let (url, handle) = serve("200 OK", r#"{"number":154,"state":"open","title":"K1"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "issue", "show", "PerishLab/keel#154"]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "154\topen\tK1");
}

#[test]
fn listed() {
    let (url, handle) = serve(
        "200 OK",
        r#"[{"number":1,"state":"open","title":"one"}]"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/keel",
        "issue",
        "list",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "1\topen\tone");
}

#[test]
fn refused() {
    let (url, handle) = serve("401 Unauthorized", r#"{"message":"unauthorized"}"#, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[":perish", "@forgejo", "user", "show"]);
    handle.join().expect("server");
    assert!(!output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stderr).contains("unauthorized"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn unknown() {
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[":", "@missing"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unknown Runseal tool: @missing"));
}

#[test]
fn token() {
    let (url, handle) = serve("200 OK", r#"{"login":"perish"}"#, 1);
    let seat = Seat::new();
    std::fs::write(
        seat.home.join("profiles/perish/runseal.toml"),
        format!("[env.vars]\nFORGEJO_URL = \"{url}\"\nFORGEJO_TOKEN = \"secret-token\"\n"),
    )
    .expect("profile");
    let output = seat.run(&[":perish", "@forgejo", "user", "show"]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "perish");
}

fn projection(path: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("PATH".into(), path.into()),
        ("RUNSEAL_FORGEJO_ISSUER_NAMESPACE".into(), "forge".into()),
        (
            "RUNSEAL_FORGEJO_ISSUER_WORKLOAD".into(),
            "deployment/forgejo".into(),
        ),
        ("RUNSEAL_FORGEJO_ISSUER_CONTAINER".into(), "forgejo".into()),
        ("RUNSEAL_FORGEJO_STORE_NAMESPACE".into(), "data".into()),
        ("RUNSEAL_FORGEJO_STORE_POD".into(), "postgres-0".into()),
        ("RUNSEAL_FORGEJO_STORE_DATABASE".into(), "forgejo".into()),
        ("RUNSEAL_FORGEJO_STORE_USER".into(), "forgejo".into()),
    ])
}

fn words(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
#[cfg(unix)]
fn issuer() {
    let fixture = tempfile::tempdir().expect("fixture");
    let command = fixture.path().join("kubectl");
    std::fs::write(
        &command,
        "#!/bin/sh\ncase \"$*\" in *generate-access-token*) printf '0123456789secret\\n' ;; *psql*) printf '1\\n' ;; *) exit 9 ;; esac\n",
    )
    .expect("fake kubectl");
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).expect("executable");
    let environment = projection(fixture.path().to_str().expect("path"));

    let create = runseal::tool::call(
        "forgejo",
        &words(&[
            "admin",
            "token",
            "create",
            "PerishFire",
            "plumb-release-registry-v1",
            "--scopes",
            "public-only,read:user,write:package",
        ]),
        &environment,
    )
    .expect("create");
    assert_eq!(
        create.secret().map(runseal::tool::Secret::expose),
        Some("0123456789secret")
    );

    let drop = runseal::tool::call(
        "forgejo",
        &words(&[
            "admin",
            "token",
            "delete",
            "PerishFire",
            "plumb-release-registry-v1",
            "42",
        ]),
        &environment,
    )
    .expect("drop");
    assert_eq!(drop.value["revoked"], true);
}

#[test]
fn injection() {
    let error = runseal::tool::call(
        "forgejo",
        &words(&[
            "admin",
            "token",
            "delete",
            "PerishFire",
            "name';delete",
            "42",
        ]),
        &projection("/unavailable"),
    )
    .expect_err("refusal");
    assert!(error.to_string().contains("simple token"));
}

#[test]
fn authority() {
    let (url, handle) = serve(
        "200 OK",
        r#"[{"id":42,"name":"plumb-release-registry-v1","scopes":["public-only","read:user","write:package"],"token_last_eight":"12345678"}]"#,
        1,
    );
    let environment = BTreeMap::from([
        ("FORGEJO_URL".into(), url),
        ("FORGEJO_TOKEN".into(), "secret-token".into()),
    ]);
    let reply = runseal::tool::call(
        "forgejo",
        &words(&["admin", "token", "list", "PerishFire"]),
        &environment,
    )
    .expect("token list");
    let seen = handle.join().expect("server");
    assert_eq!(reply.kind, "tokens");
    assert_eq!(reply.value[0]["id"], 42);
    assert!(seen[0].starts_with("GET /api/v1/users/PerishFire/tokens?limit=50&page=1 "));
}

fn passed(args: &[&str], status: &str, body: &str) -> (String, Vec<String>) {
    let (url, handle) = serve(status, body, 1);
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(args);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    (text(&output.stdout), seen)
}

#[test]
fn secrets() {
    let (out, seen) = passed(
        &[":perish", "@forgejo", "org", "secret", "list", "PerishLab"],
        "200 OK",
        r#"{"secrets":[{"name":"WORKFLOW_INVENTORY_BUCKET"}]}"#,
    );
    assert!(out.contains("WORKFLOW_INVENTORY_BUCKET"), "{out}");
    assert!(seen[0].starts_with("GET /api/v1/orgs/PerishLab/actions/secrets "));
}

#[test]
fn stored() {
    let (out, seen) = passed(
        &[
            ":perish",
            "@forgejo",
            "org",
            "secret",
            "set",
            "PerishLab",
            "T",
            "--body",
            "held",
        ],
        "204 No Content",
        "",
    );
    assert_eq!(out.trim(), "ok");
    assert!(seen[0].starts_with("PUT /api/v1/orgs/PerishLab/actions/secrets/T "));
    assert!(seen[0].contains(r#"{"data":"held"}"#), "{}", seen[0]);
}

#[test]
fn cleared() {
    let (out, seen) = passed(
        &[
            ":perish",
            "@forgejo",
            "org",
            "secret",
            "delete",
            "PerishLab",
            "T",
        ],
        "204 No Content",
        "",
    );
    assert_eq!(out.trim(), "ok");
    assert!(seen[0].starts_with("DELETE /api/v1/orgs/PerishLab/actions/secrets/T "));
}
