use super::Fixture;
use runseal::tencent::{Class, Client, Outcome, cam::Request};
use serde_json::{Value, json};

#[test]
fn custody() {
    for (credential, identity, class) in [
        (Value::Null, Value::Null, Class::Authentication),
        (
            json!({"id":"AKID_FIXTURE","key":"PRIVATE_MARKER","expires":1}),
            Value::Null,
            Class::Expired,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"PRIVATE_MARKER"}),
            Value::Null,
            Class::Binding,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"PRIVATE_MARKER"}),
            json!({"AccountId":"999","credentialStamp":"foreign"}),
            Class::Binding,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"PRIVATE_MARKER"}),
            json!({"AccountId":"123","credentialStamp":"foreign"}),
            Class::Binding,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(&json!({"version":1,"credential":credential,"identity":identity}));
        let client = Client::open(&fixture.state, "123").unwrap();
        for request in [
            Request::Entities {
                policy: 42,
                page: 1,
            },
            Request::Delete { policy: 42 },
        ] {
            let error = client.cam(&request).unwrap_err();
            assert_eq!(error.outcome, Outcome::Unsent);
            assert_eq!(error.fault.class, class);
            assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
        }
        assert!(fixture.call(&["auth", "status"]).is_err());
        drop(client);
        fixture.call(&["auth", "status"]).unwrap();
    }
}

#[test]
fn validation() {
    let fixture = Fixture::new();
    fixture.write(&json!({"version":1,"credential":null,"identity":null}));
    let client = Client::open(&fixture.state, "123").unwrap();
    for request in [
        Request::Delete { policy: 0 },
        Request::Entities { policy: 1, page: 0 },
    ] {
        let error = client.cam(&request).unwrap_err();
        assert_eq!(error.outcome, Outcome::Unsent);
        assert_eq!(error.fault.class, Class::Input);
        assert!(error.fault.request.is_none());
    }
}

#[test]
fn native() {
    let fixture = Fixture::new();
    fixture.write(&json!({"version":1,"credential":null,"identity":null}));
    for verb in ["entities", "delete"] {
        let error = fixture
            .call(&["cam", "policy", verb, "--id", "42"])
            .unwrap_err();
        assert!(error.to_string().contains("explicit owner"));
        let error = fixture
            .call(&["--account", "123", "cam", "policy", verb, "--id", "42"])
            .unwrap_err();
        assert!(!format!("{error:#?}").contains("PRIVATE_MARKER"));
        assert!(error.downcast_ref::<runseal::tencent::Error>().is_some());
    }
}
