use super::Fixture;
use runseal::tencent::{Class, Client, Product, Query, Window};
use serde_json::{Value, json};

#[test]
fn reader() {
    for (credential, identity, expected) in [
        (Value::Null, Value::Null, Class::Authentication),
        (
            json!({"id":"AKID_FIXTURE","key":"SECRET_FIXTURE","expires":1}),
            Value::Null,
            Class::Expired,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"SECRET_FIXTURE"}),
            Value::Null,
            Class::Binding,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"SECRET_FIXTURE"}),
            json!({"AccountId":"999","credentialStamp":"wrong"}),
            Class::Binding,
        ),
        (
            json!({"id":"AKID_FIXTURE","key":"SECRET_FIXTURE"}),
            json!({"AccountId":"123","credentialStamp":"wrong"}),
            Class::Binding,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(&json!({"version":1,"credential":credential,"identity":identity}));
        let client = Client::open(&fixture.state, "123").unwrap();
        let query = Query::Instances {
            product: Product::Cvm,
            region: "ap-hongkong".into(),
            window: Window {
                offset: 0,
                limit: 20,
            },
        };
        let failure = client.read(&query).unwrap_err();
        assert_eq!(failure.class, expected);
        assert!(!format!("{failure:?} {failure}").contains("SECRET_FIXTURE"));
        let operation = runseal::tencent::network::Request {
            region: "ap-hongkong".into(),
            operation: runseal::tencent::network::Operation::Vpc(
                runseal::tencent::network::Vpc::Get(runseal::tencent::network::Id {
                    id: "vpc-fixture".into(),
                }),
            ),
        };
        let failure = client.network(&operation).unwrap_err();
        assert_eq!(failure.fault.class, expected);
        assert!(matches!(
            failure.outcome,
            runseal::tencent::network::Outcome::Unsent
        ));
        assert!(!format!("{failure:?} {failure}").contains("SECRET_FIXTURE"));
        let query = runseal::tencent::dns::Request {
            zone: runseal::tencent::dns::Zone::Id(42),
            operation: runseal::tencent::dns::Operation::Get { id: 9 },
        };
        let failure = client.dns(&query).unwrap_err();
        assert_eq!(failure.fault.class, expected);
        assert_eq!(failure.outcome, runseal::tencent::Outcome::Unsent);
        assert!(!format!("{failure:?} {failure}").contains("SECRET_FIXTURE"));
        assert!(fixture.call(&["auth", "status"]).is_err());
        let query = runseal::tencent::cvm::Request {
            region: "ap-hongkong".into(),
            operation: runseal::tencent::cvm::Operation::Get {
                id: "ins-fixture".into(),
            },
        };
        let failure = client.cvm(&query).unwrap_err();
        assert_eq!(failure.fault.class, expected);
        assert_eq!(failure.outcome, runseal::tencent::Outcome::Unsent);
        assert!(!format!("{failure:?} {failure}").contains("SECRET_FIXTURE"));
        drop(client);
        assert!(fixture.call(&["auth", "status"]).is_ok());
    }
}

#[test]
fn unwind() {
    let fixture = Fixture::new();
    fixture.write(&json!({"version":1,"credential":null,"identity":null}));
    let lock = fixture.state.with_extension("lock");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let client = Client::open(&fixture.state, "123").unwrap();
        assert!(runseal::custody::Private::current().lock(&lock).is_err());
        drop(client);
        let client = Client::open(&fixture.state, "123").unwrap();
        assert!(runseal::custody::Private::current().lock(&lock).is_err());
        let _keep = client;
        panic!("session unwinds");
    }));
    assert!(result.is_err());
    let held = runseal::custody::Private::current().lock(&lock).unwrap();
    drop(held);
    Client::open(&fixture.state, "123").unwrap();
}

#[test]
fn failure() {
    let fixture = Fixture::new();
    fixture.write(&json!({"version":2,"credential":null,"identity":null}));
    assert!(Client::open(&fixture.state, "123").is_err());
    let held = runseal::custody::Private::current()
        .lock(&fixture.state.with_extension("lock"))
        .unwrap();
    drop(held);
    assert!(Client::open(&fixture.state, "123").is_err());
    assert!(
        runseal::custody::Private::current()
            .lock(&fixture.state.with_extension("lock"))
            .is_ok()
    );
}
