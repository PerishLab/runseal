use super::{caller, credential, identity as held};
use crate::alibaba::{
    api::sign,
    identity::decode,
    model::{self, Grant, Identity, State, stamp},
};

#[test]
fn observed() {
    let stamp = stamp(&credential());
    let value = caller();
    let held = decode(&value, "1234567890", &stamp).unwrap();
    assert_eq!(held.account, "1234567890");
    assert_eq!(held.principal, "1234567890123456");
    assert_eq!(held.user, "2345678901234567");
    assert_eq!(held.request, "fixture-request-id");
    assert_eq!(held.stamp, stamp);
    let mut equal = value.clone();
    equal["UserId"] = equal["PrincipalId"].clone();
    let same = decode(&equal, "1234567890", &stamp).unwrap();
    assert_eq!(same.principal, same.user);
}

#[test]
fn refused() {
    let stamp = stamp(&credential());
    let base = caller();
    let mut cases: Vec<serde_json::Value> = Vec::new();
    let mut case = base.clone();
    case["IdentityType"] = "Account".into();
    cases.push(case);
    let mut case = base.clone();
    case["IdentityType"] = "AssumedRoleUser".into();
    cases.push(case);
    let mut case = base.clone();
    case["AccountId"] = "1234567891".into();
    case["Arn"] = "acs:ram::1234567891:user/fixture-user".into();
    cases.push(case);
    let mut case = base.clone();
    case["Arn"] = "acs:ram::1234567890:role/fixture-role".into();
    cases.push(case);
    let mut case = base.clone();
    case["Arn"] = "acs:ram::1234567890:user/".into();
    cases.push(case);
    let mut case = base.clone();
    case["Arn"] = format!("acs:ram::1234567890:user/{}", "a".repeat(520)).into();
    cases.push(case);
    let mut case = base.clone();
    case["Arn"] = "acs:ram::1234567890:user/fixture\u{fc}".into();
    cases.push(case);
    let mut case = base.clone();
    case["AccountId"] = "0123456789".into();
    cases.push(case);
    let mut case = base.clone();
    case["PrincipalId"] = "fixture-principal".into();
    cases.push(case);
    let mut case = base.clone();
    case["UserId"] = "0".into();
    cases.push(case);
    let mut case = base.clone();
    case["RequestId"] = "fixture\u{7f}".into();
    cases.push(case);
    let mut case = base.clone();
    case["RequestId"] = "fixture/request".into();
    cases.push(case);
    let mut case = base.clone();
    case["RequestId"] = format!("fixture-{}", "a".repeat(140)).into();
    cases.push(case);
    for value in cases {
        assert!(decode(&value, "1234567890", &stamp).is_err(), "{value}");
    }
    for key in [
        "AccountId",
        "PrincipalId",
        "UserId",
        "IdentityType",
        "Arn",
        "RequestId",
    ] {
        let mut value = base.clone();
        value.as_object_mut().unwrap().remove(key);
        assert!(decode(&value, "1234567890", &stamp).is_err(), "{key}");
    }
    assert!(decode(&base, "1234567891", &stamp).is_err());
    assert!(decode(&base, "1234567890", "fixture-stamp").is_err());
}

#[test]
fn validated() {
    let verified = held();
    verified.validate().unwrap();
    let mut case = held();
    case.stamp = "g".repeat(64);
    assert!(case.validate().is_err());
    let mut case = held();
    case.stamp = "a".repeat(63);
    assert!(case.validate().is_err());
    let mut case = held();
    case.kind = "Account".into();
    assert!(case.validate().is_err());
    let mut case = held();
    case.arn = "acs:ram::1234567891:user/fixture-user".into();
    assert!(case.validate().is_err());
    let mut case = held();
    case.arn = "acs:ram::1234567890:user/".into();
    assert!(case.validate().is_err());
    let mut case = held();
    case.account = "123456789a".into();
    assert!(case.validate().is_err());
}

#[test]
fn invalidated() {
    let mut state = State::fresh();
    state.credential = Some(credential());
    state.identity = Some(held());
    state.check().unwrap();
    state.credential.as_mut().unwrap().key = "PRIVATE_OTHER_KEY".into();
    assert!(state.check().is_err());
    state.credential = None;
    assert!(state.check().is_err());
}

#[test]
fn receipt() {
    let mut state = State::fresh();
    state.oauth = Some(Grant {
        access: "PRIVATE_ACCESS_MARKER".into(),
        refresh: "PRIVATE_REFRESH_MARKER".into(),
        expires: model::now() + 60,
        observed: 1,
    });
    state.credential = Some(credential());
    state.identity = Some(held());
    let receipt = state.receipt();
    let text = receipt.to_string();
    assert!(!text.contains("PRIVATE"));
    assert_eq!(receipt["authorized"], true);
    assert_eq!(receipt["credentialPresent"], true);
    assert_eq!(receipt["identity"]["principal"], "1234567890123456");
    assert_eq!(receipt["identity"]["kind"], "RAMUser");
    assert!(receipt["pending"].is_null());
}

#[test]
fn encoded() {
    let held: Identity = serde_json::from_value(serde_json::to_value(held()).unwrap()).unwrap();
    assert_eq!(held.stamp, stamp(&credential()));
    assert_eq!(sign::hash(b"fixture").len(), 64);
}
