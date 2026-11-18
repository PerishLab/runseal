use super::{Fixture, base64};
use crate::alibaba::{
    Auth, Outcome,
    model::{self, Pending, Retained, State},
};
use serde_json::{Value, json};

fn fixture(stage: &str, status: u16, value: &Value) -> Fixture {
    let fixture = Fixture::new();
    let mut state = State::fresh();
    state.pending = Some(Pending {
        stage: stage.into(),
        started: model::now() - 10,
    });
    state.retained = Some(Retained {
        stage: stage.into(),
        status,
        body: base64(&serde_json::to_vec(value).unwrap()),
    });
    fixture.store().create(&state).unwrap();
    fixture
}

#[test]
fn oauth() {
    let value = json!({"access_token":"PRIVATE_ACCESS_MARKER","refresh_token":"PRIVATE_REFRESH_MARKER","token_type":"Bearer","expires_in":60});
    let fixture = fixture(model::CODE, 200, &value);
    let started = fixture.store().read().unwrap().pending.unwrap().started;
    let auth = Auth::new(&fixture.state).unwrap();
    let receipt = auth.recover().unwrap();
    assert_eq!(receipt["authorized"], true);
    assert_eq!(receipt["credentialPresent"], false);
    assert_eq!(receipt["oauthExpiresAt"], started + 60);
    assert!(receipt["identity"].is_null());
    assert!(receipt["pending"].is_null());
    assert!(!receipt.to_string().contains("PRIVATE_"));
    assert_eq!(auth.recover().unwrap_err().outcome, Outcome::Unsent);
}

#[test]
fn sts() {
    for value in [
        credential(),
        json!({"accessKeyId":"STS.fixture-id","accessKeySecret":"PRIVATE_KEY_MARKER","securityToken":"PRIVATE_TOKEN_MARKER","expiration":"2999-01-01T00:00:00Z"}),
    ] {
        let fixture = fixture(model::STS, 200, &value);
        let started = fixture.store().read().unwrap().pending.unwrap().started;
        let auth = Auth::new(&fixture.state).unwrap();
        let receipt = auth.recover().unwrap();
        assert_eq!(receipt["credentialPresent"], true);
        assert!(receipt["identity"].is_null());
        assert!(receipt["pending"].is_null());
        assert!(!receipt.to_string().contains("PRIVATE_"));
        assert_eq!(auth.recover().unwrap_err().outcome, Outcome::Unsent);
        drop(auth);
        let state = fixture.store().read().unwrap();
        assert_eq!(state.credential.unwrap().observed, started);
        assert!(state.retained.is_none());
    }
}

fn credential() -> Value {
    json!({"AccessKeyId":"STS.fixture-id","AccessKeySecret":"PRIVATE_KEY_MARKER","SecurityToken":"PRIVATE_TOKEN_MARKER","Expiration":"2999-01-01T00:00:00Z"})
}

fn refuse(value: &Value) {
    let fixture = fixture(model::STS, 200, value);
    let original = std::fs::read(&fixture.state).unwrap();
    let auth = Auth::new(&fixture.state).unwrap();
    let error = auth.recover().unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(!error.to_string().contains("PRIVATE_"));
    assert_eq!(auth.status().unwrap()["pending"]["retainedReturn"], true);
    assert_eq!(auth.status().unwrap()["credentialPresent"], false);
    assert_eq!(auth.renew().unwrap_err().outcome, Outcome::Unknown);
    drop(auth);
    assert_eq!(std::fs::read(&fixture.state).unwrap(), original);
}

#[test]
fn invalid() {
    for (field, value) in [
        ("Expiration", json!("2000-01-01T00:00:00Z")),
        ("Expiration", Value::Null),
        ("AccessKeyId", json!("")),
        ("AccessKeySecret", json!(42)),
        ("SecurityToken", Value::Null),
    ] {
        let mut body = credential();
        body[field] = value;
        refuse(&body);
    }
    let mut missing = credential();
    missing.as_object_mut().unwrap().remove("SecurityToken");
    refuse(&missing);
    let mut casing = credential();
    let value = casing
        .as_object_mut()
        .unwrap()
        .remove("AccessKeyId")
        .unwrap();
    casing["ACCESSKEYID"] = value;
    refuse(&casing);
}

#[test]
fn ambiguous() {
    for (canonical, alternate) in [
        ("AccessKeyId", "accessKeyId"),
        ("AccessKeySecret", "accessKeySecret"),
        ("SecurityToken", "securityToken"),
        ("Expiration", "expiration"),
    ] {
        let mut body = credential();
        body[alternate] = body[canonical].clone();
        refuse(&body);
    }
}

#[test]
fn uncertain() {
    for (status, body) in [
        (
            500,
            json!({"error":"invalid_grant","error_description":"PRIVATE_PROVIDER_MARKER"}),
        ),
        (
            200,
            json!({"access_token":"PRIVATE_ACCESS_MARKER","refresh_token":"PRIVATE_REFRESH_MARKER","token_type":"Bearer","expires_in":"60"}),
        ),
        (
            400,
            json!({"error":"unrecognized","error_description":"PRIVATE_PROVIDER_MARKER"}),
        ),
    ] {
        let fixture = fixture(model::CODE, status, &body);
        let auth = Auth::new(&fixture.state).unwrap();
        let error = auth.recover().unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(!error.to_string().contains("PRIVATE_"));
        assert_eq!(auth.status().unwrap()["pending"]["retainedReturn"], true);
        assert_eq!(auth.renew().unwrap_err().outcome, Outcome::Unknown);
    }
}

#[test]
fn rejected() {
    let fixture = fixture(
        model::CODE,
        400,
        &json!({"error":"invalid_grant","error_description":"PRIVATE_PROVIDER_MARKER"}),
    );
    let auth = Auth::new(&fixture.state).unwrap();
    let error = auth.recover().unwrap_err();
    assert_eq!(error.outcome, Outcome::Rejected);
    assert!(!error.to_string().contains("PRIVATE_"));
    assert!(auth.status().unwrap()["pending"].is_null());
}
