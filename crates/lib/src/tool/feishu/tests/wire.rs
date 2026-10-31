use super::*;
use serde_json::json;
use std::collections::BTreeMap;
use zeroize::Zeroizing;

fn payload(status: u16, value: Value) -> wire::Payload {
    wire::Payload {
        status,
        headers: BTreeMap::from([(
            "x-tt-logid".into(),
            Zeroizing::new("20261030FIXTURE".into()),
        )]),
        body: Zeroizing::new(serde_json::to_vec(&value).unwrap()),
    }
}

#[test]
fn refusal() {
    struct Case {
        status: u16,
        code: i64,
        class: Class,
        outcome: super::super::Outcome,
    }
    for case in [
        Case {
            status: 400,
            code: 20002,
            class: Class::Authentication,
            outcome: super::super::Outcome::Rejected,
        },
        Case {
            status: 200,
            code: 99991672,
            class: Class::Permission,
            outcome: super::super::Outcome::Rejected,
        },
        Case {
            status: 200,
            code: 987654,
            class: Class::Response,
            outcome: super::super::Outcome::Unknown,
        },
        Case {
            status: 500,
            code: 20002,
            class: Class::Response,
            outcome: super::super::Outcome::Unknown,
        },
    ] {
        let error = decode(payload(
            case.status,
            json!({"code":case.code,"msg":"SECRET_FIXTURE","error_description":"SECRET_FIXTURE"}),
        ))
        .err()
        .unwrap();
        assert_eq!(error.class, case.class);
        assert_eq!(error.outcome, case.outcome);
        assert_eq!(error.request.as_deref(), Some("20261030FIXTURE"));
        assert!(!format!("{error:?} {error}").contains("SECRET_FIXTURE"));
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("SECRET_FIXTURE")
        );
    }
}

#[test]
fn malformed() {
    for value in [
        json!({}),
        json!({"code":"0"}),
        json!({"code":null}),
        json!([]),
    ] {
        assert_eq!(
            decode(payload(200, value)).err().unwrap().outcome,
            super::super::Outcome::Unknown
        );
    }
    let mut held = payload(200, json!({"code":0}));
    held.body = Zeroizing::new(b"SECRET_FIXTURE".to_vec());
    assert!(
        !decode(held)
            .err()
            .unwrap()
            .to_string()
            .contains("SECRET_FIXTURE")
    );
}

#[test]
fn success() {
    let held = decode(payload(200, json!({"code":0,"data":{}}))).unwrap();
    assert_eq!(held.body.0["code"], 0);
}

#[test]
fn oauth() {
    for (name, poll) in [
        ("authorization_pending", super::super::Poll::Pending),
        ("slow_down", super::super::Poll::Slow),
    ] {
        let error = authorization(payload(
            400,
            json!({"error":name,"error_description":"SECRET_FIXTURE"}),
        ))
        .err()
        .unwrap();
        assert_eq!(error.poll, Some(poll));
        assert_eq!(error.outcome, super::super::Outcome::Rejected);
        assert!(!format!("{error:?}").contains("SECRET"));
    }
    for status in [200, 400, 500] {
        let error = authorization(payload(
            status,
            json!({"error":"UNKNOWN_SECRET","error_description":"SECRET_FIXTURE"}),
        ))
        .err()
        .unwrap();
        assert_eq!(error.outcome, super::super::Outcome::Unknown);
        assert!(error.poll.is_none());
        assert!(!format!("{error:?}").contains("SECRET"));
    }
    assert!(authorization(payload(200, json!({"access_token":"SECRET_FIXTURE"}))).is_ok());
    let error = authorization(payload(500, json!({"error":"authorization_pending"})))
        .err()
        .unwrap();
    assert!(error.poll.is_none());
    assert_eq!(error.outcome, super::super::Outcome::Unknown);
}
