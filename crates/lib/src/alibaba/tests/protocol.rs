use crate::alibaba::{api::sign, oauth};

#[test]
fn pkce() {
    assert_eq!(
        oauth::challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let verifier = oauth::verifier().unwrap();
    assert!((43..=128).contains(&verifier.len()));
    assert!(
        verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    );
}

#[test]
fn authorize() {
    let url = oauth::authorize("fixture-state", "fixture-challenge").unwrap();
    assert!(url.starts_with("https://signin.aliyun.com/oauth2/v1/auth?"));
    for required in [
        "client_id=4038181954557748008",
        "response_type=code",
        "code_challenge_method=S256",
        "code_challenge=fixture-challenge",
        "state=fixture-state",
        "redirect_uri=http%3A%2F%2F127.0.0.1%3A12345%2Fcli%2Fcallback",
    ] {
        assert!(url.contains(required), "{required} missing from {url}");
    }
}

#[test]
fn callback() {
    let digest = sign::hash(b"fixture-state");
    let good = format!("{}?code=fixture-code&state=fixture-state", oauth::REDIRECT);
    assert_eq!(oauth::callback(&good, &digest).unwrap(), "fixture-code");
    assert!(oauth::callback(&good, "fixture-state").is_err());
    assert!(oauth::callback(&good, &sign::hash(b"other-state")).is_err());
    for rejected in [
        "http://127.0.0.1:12345/other?code=c&state=fixture-state",
        "https://127.0.0.1:12345/cli/callback?code=c&state=fixture-state",
        "http://127.0.0.1:12346/cli/callback?code=c&state=fixture-state",
        "http://localhost:12345/cli/callback?code=c&state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?code=a&code=b&state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?code=c&state=fixture-state&state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?error=access_denied&state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?code=&state=fixture-state",
        "http://127.0.0.1:12345/cli/callback?code=c&state=fixture-state#fragment",
        "user@127.0.0.1:12345/cli/callback?code=c&state=fixture-state",
    ] {
        assert!(oauth::callback(rejected, &digest).is_err(), "{rejected}");
    }
}

#[test]
fn form() {
    assert_eq!(
        oauth::form(&[
            ("grant_type", "authorization_code"),
            ("redirect_uri", oauth::REDIRECT),
        ])
        .as_str(),
        "grant_type=authorization_code&redirect_uri=http%3A%2F%2F127.0.0.1%3A12345%2Fcli%2Fcallback"
    );
}
