use super::*;

fn refused(violations: Value) -> Error {
    decode(payload(
        400,
        json!({"code":99991672,"msg":"SECRET_FIXTURE","error":{
            "permission_violations":violations,"helps":[{"url":"SECRET_FIXTURE"}]
        }}),
    ))
    .err()
    .unwrap()
}

#[test]
fn subjects() {
    let error = refused(json!([
        {"subject":"im:message:readonly","description":"SECRET_FIXTURE"},
        {"subject":"im:message"},
        {"subject":"im:message:readonly"},
        {"subject":"SECRET_FIXTURE"},
        {"subject":"im:message:readonly SECRET_FIXTURE"},
        {"subject":"im:message:readonly\u{0}SECRET_FIXTURE"},
        {"subject":"im:unknown:SECRET_FIXTURE"},
        {"subject":["im:message:readonly"]},
        {"subject":null},
        {"description":"im:message:readonly SECRET_FIXTURE"},
        "SECRET_FIXTURE"
    ]));
    let permissions = error.permissions.as_ref().unwrap();
    assert_eq!(permissions.subjects, ["im:message", "im:message:readonly"]);
    assert_eq!(permissions.unknown, 8);
    assert!(!permissions.truncated);
    assert_eq!(error.class, Class::Permission);
    assert_eq!(error.outcome, super::super::super::Outcome::Rejected);
    assert_eq!(error.request.as_deref(), Some("20261030FIXTURE"));
    assert!(!format!("{error:?} {error}").contains("SECRET_FIXTURE"));
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("SECRET_FIXTURE")
    );
    assert!(
        error
            .to_string()
            .contains("subjects alone do not establish")
    );
}

#[test]
fn limits() {
    let mut items = vec![json!({"subject":"SECRET_FIXTURE"}); 32];
    items.push(json!({"subject":"im:message:readonly"}));
    let error = refused(json!(items));
    let permissions = error.permissions.unwrap();
    assert!(permissions.subjects.is_empty());
    assert_eq!(permissions.unknown, 32);
    assert!(permissions.truncated);
}

#[test]
fn shapes() {
    for value in [Value::Null, json!({}), json!("SECRET_FIXTURE")] {
        let error = refused(value);
        let permissions = error.permissions.as_ref().unwrap();
        assert!(permissions.subjects.is_empty());
        assert_eq!(permissions.unknown, 1);
        assert!(!permissions.truncated);
        assert!(!format!("{error:?} {error}").contains("SECRET_FIXTURE"));
    }
    let empty = refused(json!([]));
    assert_eq!(empty.permissions.unwrap().unknown, 0);
    let mut value = json!({"code":0,"error":{
        "permission_violations":[{"subject":"im:message:readonly"}]
    }});
    for (status, code) in [(400, 20002), (500, 99991672)] {
        value["code"] = json!(code);
        let error = decode(payload(status, value.clone())).err().unwrap();
        assert!(error.permissions.is_none());
    }
    let absent = decode(payload(400, json!({"code":99991672})))
        .err()
        .unwrap();
    assert!(absent.permissions.is_none());
}
