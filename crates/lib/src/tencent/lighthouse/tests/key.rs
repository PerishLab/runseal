use super::*;

fn observe(row: Value) -> Key {
    let Data::Key(Lookup {
        value: Some(key),
        total: 1,
        complete: true,
        ..
    }) = super::key()
        .decode(&fixture::page("KeyPairSet", row))
        .unwrap()
    else {
        panic!("key")
    };
    key
}

#[test]
fn associations() {
    let value = observe(fixture::key());
    assert_eq!(value.instances, Some(vec!["lhins-fixture".into()]));
    let associations = value.associations.unwrap();
    assert_eq!(associations.len(), 2);
    assert_eq!(associations[0].user.as_deref(), Some("root"));
    assert_eq!(associations[1].user.as_deref(), Some("ubuntu"));
    assert_eq!(value.public.as_deref(), Some(""));
    let mut row = fixture::key();
    row["AssociatedInstanceIds"] = json!([]);
    assert_eq!(observe(row).associations.unwrap().len(), 2);
    let encoded = serde_json::to_string(&observe(fixture::key())).unwrap();
    assert!(!encoded.contains("PRIVATE_MARKER"));
}

#[test]
fn public() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let bytes = [
        b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20".as_slice(),
        &[1u8; 32],
    ]
    .concat();
    let public = format!("ssh-ed25519 {}", STANDARD.encode(bytes));
    let mut row = fixture::key();
    row["PublicKey"] = json!(public);
    assert_eq!(observe(row).public, Some(public));
    for invalid in [
        "PRIVATE_MARKER",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "ssh-rsa invalid",
    ] {
        let mut row = fixture::key();
        row["PublicKey"] = json!(invalid);
        let failure = super::key()
            .decode(&fixture::page("KeyPairSet", row))
            .unwrap_err();
        assert!(!format!("{failure:?} {failure}").contains(invalid));
    }
}

#[test]
fn nullable() {
    for field in [
        "KeyName",
        "PublicKey",
        "AssociatedInstanceIds",
        "AssociatedInstanceSet",
        "CreatedTime",
    ] {
        for null in [true, false] {
            let mut row = fixture::key();
            if null {
                row[field] = Value::Null
            } else {
                row.as_object_mut().unwrap().remove(field);
            }
            let key = observe(row);
            match field {
                "PublicKey" => assert!(key.public.is_none()),
                "AssociatedInstanceIds" => assert!(key.instances.is_none()),
                "AssociatedInstanceSet" => assert!(key.associations.is_none()),
                _ => (),
            }
        }
    }
    let mut row = fixture::key();
    row["AssociatedInstanceSet"] = json!([]);
    assert_eq!(observe(row).associations.unwrap().len(), 0);
    for value in [Value::Null, json!("")] {
        let mut row = fixture::key();
        row["AssociatedInstanceSet"] = json!([{"InstanceId":"lhins-fixture","Username":value}]);
        assert_eq!(observe(row).associations.unwrap().len(), 1);
    }
}

#[test]
fn conflicts() {
    for (field, value) in [
        ("KeyId", json!("lhkp-foreign")),
        ("AssociatedInstanceIds", json!(["ins-wrong"])),
        (
            "AssociatedInstanceIds",
            json!(["lhins-fixture", "lhins-fixture"]),
        ),
        ("AssociatedInstanceSet", json!([{"InstanceId":"ins-wrong"}])),
        (
            "AssociatedInstanceSet",
            json!([{"InstanceId":"lhins-fixture","Username":"root"},{"InstanceId":"lhins-fixture","Username":"root"}]),
        ),
        ("AssociatedInstanceSet", json!([{"Username":"root"}])),
        ("CreatedTime", json!("bad")),
    ] {
        let mut row = fixture::key();
        row[field] = value;
        assert!(
            super::key()
                .decode(&fixture::page("KeyPairSet", row))
                .is_err(),
            "{field}"
        );
    }
    let mut row = fixture::key();
    row["AssociatedInstanceSet"][0]["Future"] = json!("PRIVATE_MARKER");
    let data = super::key()
        .decode(&fixture::page("KeyPairSet", row))
        .unwrap();
    assert!(matches!(
        data,
        Data::Key(Lookup {
            complete: false,
            ..
        })
    ));
    assert!(
        !serde_json::to_string(&data)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    assert!(matches!(
        super::key()
            .decode(&json!({"KeyPairSet":[],"TotalCount":0}))
            .unwrap(),
        Data::Key(Lookup {
            value: None,
            total: 0,
            complete: true,
            ..
        })
    ));
}

#[test]
fn pagination() {
    let mut body = fixture::page("KeyPairSet", fixture::key());
    body["TotalCount"] = json!(3);
    let Data::Keys(page) = super::keys().decode(&body).unwrap() else {
        panic!("page")
    };
    assert_eq!(page.next, Some(1));
    assert_eq!(page.total, 3);
    assert!(super::key().decode(&body).is_err());
    let request = request(Operation::Keys(Window {
        offset: 2,
        limit: 1,
    }));
    let Data::Keys(page) = request.decode(&body).unwrap() else {
        panic!("page")
    };
    assert_eq!(page.next, None);
    for body in [
        json!({"KeyPairSet":[],"TotalCount":3}),
        json!({"KeyPairSet":null,"TotalCount":0}),
        json!({"KeyPairSet":[fixture::key()],"TotalCount":0}),
        json!({"KeyPairSet":[fixture::key(),fixture::key()],"TotalCount":2}),
        json!({"KeyPairSet":[],"TotalCount":-1}),
        json!({"KeyPairSet":[]}),
    ] {
        assert!(super::keys().decode(&body).is_err());
    }
    let mut body = fixture::page("KeyPairSet", fixture::key());
    body["Future"] = json!("PRIVATE_MARKER");
    let Data::Keys(page) = super::keys().decode(&body).unwrap() else {
        panic!("page")
    };
    assert!(!page.complete);
    assert!(
        !serde_json::to_string(&page)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let request = super::request(Operation::Keys(Window {
        offset: u32::MAX,
        limit: 1,
    }));
    body["TotalCount"] = json!(u64::from(u32::MAX) + 2);
    assert!(request.decode(&body).is_err());
}
