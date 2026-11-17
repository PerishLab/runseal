use super::{Fixture, base64, credential};
use crate::alibaba::{
    Client,
    model::{self, BODY, Pending, RETAINED, Retained, STATE, State},
    secret::Store,
};

#[test]
fn roundtrip() {
    let fixture = Fixture::new();
    let mut state = State::fresh();
    state.credential = Some(credential());
    fixture.store().create(&state).unwrap();
    let held = fixture.store().read().unwrap();
    assert_eq!(held.version, 1);
    assert_eq!(held.credential.unwrap().key, "PRIVATE_KEY_MARKER");
    let mut change = fixture.store().read().unwrap();
    change.oauth = Some(model::Grant {
        access: "PRIVATE_ACCESS_MARKER".into(),
        refresh: "PRIVATE_REFRESH_MARKER".into(),
        expires: model::now() + 60,
        observed: 1,
    });
    fixture.store().save(&change).unwrap();
    assert_eq!(
        fixture.store().read().unwrap().oauth.unwrap().expires,
        change.oauth.unwrap().expires
    );
    assert!(fixture.store().create(&State::fresh()).is_err());
}

#[test]
fn custody() {
    let fixture = Fixture::new();
    let held = Store::open(&fixture.state).unwrap();
    assert!(Store::open(&fixture.state).is_err());
    assert!(Store::open(std::path::Path::new("relative.json")).is_err());
    drop(held);
    drop(Store::open(&fixture.state).unwrap());
    crate::custody::Private::current()
        .create(&fixture.state, &vec![b' '; STATE + 1])
        .unwrap();
    assert!(fixture.store().read().is_err());
    crate::custody::Private::current()
        .replace(&fixture.state, b"{}")
        .unwrap();
    assert!(fixture.store().read().is_err());
}

#[test]
fn retained() {
    let fixture = Fixture::new();
    let mut state = State::fresh();
    state.pending = Some(Pending {
        stage: model::STS.into(),
        started: model::now(),
    });
    state.retained = Some(Retained {
        stage: model::STS.into(),
        status: 500,
        body: base64(&[0xff, 0xfe, 0x00, 0x41]),
    });
    fixture.store().create(&state).unwrap();
    use base64::Engine;
    let held = fixture.store().read().unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(held.retained.as_ref().unwrap().body.as_bytes())
        .unwrap();
    assert_eq!(decoded, vec![0xff, 0xfe, 0x00, 0x41]);
}

#[test]
fn refused() {
    let mut state = State::fresh();
    state.pending = Some(Pending {
        stage: "fixture-stage".into(),
        started: model::now(),
    });
    assert!(state.check().is_err());
    state.pending = Some(Pending {
        stage: model::CODE.into(),
        started: 0,
    });
    assert!(state.check().is_err());
    state.pending = None;
    state.retained = Some(Retained {
        stage: model::CODE.into(),
        status: 500,
        body: base64(b"fixture"),
    });
    assert!(state.check().is_err());
    state.pending = Some(Pending {
        stage: model::REFRESH.into(),
        started: model::now(),
    });
    assert!(state.check().is_err());
    state.pending = Some(Pending {
        stage: model::CODE.into(),
        started: model::now(),
    });
    state.check().unwrap();
    state.retained.as_mut().unwrap().body = "not base64!".into();
    assert!(state.check().is_err());
    state.retained.as_mut().unwrap().body = "Z".repeat(RETAINED + 4);
    assert!(state.check().is_err());
    use base64::Engine;
    let oversized = base64::engine::general_purpose::STANDARD.encode(vec![b'a'; BODY + 1]);
    state.retained.as_mut().unwrap().body = oversized;
    assert!(state.check().is_err());
    state.retained = None;
    state.version = 2;
    assert!(state.check().is_err());
    state.version = 1;
    state.site = "US".into();
    assert!(state.check().is_err());
}

#[test]
fn expiry() {
    let mut state = State::fresh();
    assert!(state.credential().is_err());
    let mut held = credential();
    held.expires = model::now() - 1;
    state.credential = Some(held);
    assert!(state.credential().is_err());
    state.credential = Some(credential());
    assert!(state.credential().is_ok());
}

#[test]
fn client() {
    let fixture = Fixture::new();
    fixture.store().create(&State::fresh()).unwrap();
    for account in ["", "fixture", "0123456789", "0"] {
        assert!(
            Client::open(&fixture.state, account).err().is_some(),
            "{account}"
        );
    }
    let client = Client::open(&fixture.state, "1234567890").unwrap();
    assert_eq!(client.account(), "1234567890");
    drop(client);
    assert!(
        Client::open(std::path::Path::new("state.json"), "1234567890")
            .err()
            .is_some()
    );
}
