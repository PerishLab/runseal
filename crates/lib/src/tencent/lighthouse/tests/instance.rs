use super::*;

fn observe(value: Value) -> Box<Instance> {
    let Data::Instance(Lookup {
        value: Some(instance),
        complete: true,
        total: 1,
        ..
    }) = super::instance()
        .decode(&fixture::page("InstanceSet", value))
        .unwrap()
    else {
        panic!("instance")
    };
    instance
}

#[test]
fn facts() {
    let value = observe(fixture::instance());
    assert!(value.complete);
    assert_eq!(value.zone.as_deref(), Some("na-ashburn-1"));
    assert_eq!(value.state.as_ref().map(Label::value), Some("RUNNING"));
    assert_eq!(value.platform, Some(Platform::Linux));
    assert_eq!(value.progress.stage, Some(Stage::Success));
    assert_eq!(
        value.progress.action.as_ref().map(Label::value),
        Some("AssociateInstancesKeyPairs")
    );
    assert_eq!(value.progress.request.as_deref(), Some("operation-id"));
    assert_eq!(value.login.unwrap().keys, Some(vec!["lhkp-fixture".into()]));
    assert!(value.addresses.public.unwrap()[0].is_ipv4());
    assert!(value.addresses.ipv6.unwrap()[0].is_ipv6());
}

#[test]
fn nullable() {
    for field in [
        "Uuid",
        "CreatedTime",
        "Zone",
        "InstanceState",
        "PlatformType",
        "PublicAddresses",
        "PrivateAddresses",
        "PublicIpv6Addresses",
        "LoginSettings",
        "LatestOperation",
        "LatestOperationState",
        "LatestOperationRequestId",
        "LatestOperationStartedTime",
    ] {
        for null in [true, false] {
            let mut row = fixture::instance();
            if null {
                row[field] = Value::Null
            } else {
                row.as_object_mut().unwrap().remove(field);
            }
            let value = observe(row);
            match field {
                "PublicAddresses" => assert!(value.addresses.public.is_none()),
                "PrivateAddresses" => assert!(value.addresses.private.is_none()),
                "LoginSettings" => assert!(value.login.is_none()),
                "LatestOperationState" => assert!(value.progress.stage.is_none()),
                _ => (),
            }
        }
    }
    let mut row = fixture::instance();
    row["LoginSettings"]["KeyIds"] = Value::Null;
    assert!(observe(row).login.unwrap().keys.is_none());
    let mut row = fixture::instance();
    row["LoginSettings"]["KeyIds"] = json!([]);
    row["PublicAddresses"] = json!([]);
    row["LatestOperationState"] = json!("");
    row["LatestOperationRequestId"] = json!("");
    row["LatestOperationStartedTime"] = json!("");
    let value = observe(row);
    assert_eq!(value.login.unwrap().keys, Some(vec![]));
    assert_eq!(value.addresses.public, Some(vec![]));
    assert_eq!(value.progress.stage, Some(Stage::Unavailable));
    assert_eq!(value.progress.started.as_deref(), Some(""));
}

#[test]
fn coverage() {
    for field in [
        "InstanceState",
        "PlatformType",
        "LatestOperation",
        "LatestOperationState",
    ] {
        let mut row = fixture::instance();
        row[field] = json!("PRIVATE_MARKER");
        let data = super::instance()
            .decode(&fixture::page("InstanceSet", row))
            .unwrap();
        let encoded = serde_json::to_string(&data).unwrap();
        assert!(!encoded.contains("PRIVATE_MARKER"));
        assert!(matches!(
            data,
            Data::Instance(Lookup {
                complete: false,
                ..
            })
        ));
    }
    for field in ["Foreign", "LoginSettings"] {
        let mut row = fixture::instance();
        if field == "LoginSettings" {
            row[field]["Password"] = json!("PRIVATE_MARKER")
        } else {
            row[field] = json!("PRIVATE_MARKER")
        }
        let data = super::instance()
            .decode(&fixture::page("InstanceSet", row))
            .unwrap();
        assert!(matches!(
            data,
            Data::Instance(Lookup {
                complete: false,
                ..
            })
        ));
        assert!(
            !serde_json::to_string(&data)
                .unwrap()
                .contains("PRIVATE_MARKER")
        );
    }
}

#[test]
fn schema() {
    let mut row = fixture::instance();
    row["Foreign"] = json!({"Password":"PRIVATE_MARKER"});
    row["PRIVATE_MARKER"] = json!("PRIVATE_MARKER");
    row["bad\nname"] = json!("PRIVATE_MARKER");
    let Data::Instance(lookup) = super::instance()
        .decode(&fixture::page("InstanceSet", row))
        .unwrap()
    else {
        panic!("instance")
    };
    let value = lookup.value.unwrap();
    assert!(!lookup.complete && !value.complete);
    assert_eq!(value.schema.fields, vec!["Foreign"]);
    assert_eq!(value.schema.omitted, 2);
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let mut row = fixture::instance();
    for index in 0..100 {
        row[format!("Field{index}")] = json!("PRIVATE_MARKER")
    }
    let Data::Instance(lookup) = super::instance()
        .decode(&fixture::page("InstanceSet", row))
        .unwrap()
    else {
        panic!("instance")
    };
    let value = lookup.value.unwrap();
    assert_eq!(value.schema.fields.len(), 16);
    assert_eq!(value.schema.omitted, 84);
    assert!(!lookup.complete);
}

#[test]
fn extensions() {
    for field in [
        "ActivityLabels",
        "BlueprintImageUrl",
        "DisplayArea",
        "DisplayLabel",
    ] {
        for (raw, shape) in [
            (Value::Null, Shape::Null),
            (json!(true), Shape::Boolean),
            (json!(1), Shape::Number),
            (json!("PRIVATE_MARKER"), Shape::String),
            (json!(["PRIVATE_MARKER"]), Shape::Array),
            (json!({"Password":"PRIVATE_MARKER"}), Shape::Object),
        ] {
            let mut row = fixture::instance();
            row[field] = raw;
            let value = observe(row);
            assert!(value.schema.fields.is_empty() && value.schema.omitted == 0);
            assert_eq!(value.extensions.len(), 1);
            assert_eq!(value.extensions[0].shape, shape);
            assert!(
                !serde_json::to_string(&value)
                    .unwrap()
                    .contains("PRIVATE_MARKER")
            );
        }
    }
    assert!(observe(fixture::instance()).extensions.is_empty());
    let mut row = fixture::instance();
    row["FutureExtension"] = json!("PRIVATE_MARKER");
    let Data::Instance(lookup) = super::instance()
        .decode(&fixture::page("InstanceSet", row))
        .unwrap()
    else {
        panic!("instance")
    };
    assert!(!lookup.complete);
    assert_eq!(lookup.value.unwrap().schema.fields, vec!["FutureExtension"]);
}

#[test]
fn conflicts() {
    for (field, value) in [
        ("InstanceId", json!("lhins-foreign")),
        ("Zone", json!("ap-hongkong-1")),
        ("PublicIpAddresses", Value::Null),
        ("PrivateIpAddresses", json!([])),
        ("PublicAddresses", json!(["2001:db8::1"])),
        ("PublicAddresses", json!(["192.0.2.1", "192.0.2.1"])),
        ("PrivateAddresses", json!(["bad"])),
        ("PublicIpv6Addresses", json!(["192.0.2.1"])),
        ("CreatedTime", json!("bad")),
        ("LatestOperationRequestId", json!("bad request")),
        ("LoginSettings", json!({"KeyIds":["skey-foreign"]})),
    ] {
        let mut row = fixture::instance();
        row[field] = value;
        assert!(
            super::instance()
                .decode(&fixture::page("InstanceSet", row))
                .is_err(),
            "{field}"
        );
    }
    for body in [
        json!({"InstanceSet":[],"TotalCount":1}),
        json!({"InstanceSet":null,"TotalCount":0}),
        json!({"InstanceSet":[fixture::instance(),fixture::instance()],"TotalCount":2}),
    ] {
        assert!(super::instance().decode(&body).is_err());
    }
    assert!(matches!(
        super::instance()
            .decode(&json!({"InstanceSet":[],"TotalCount":0}))
            .unwrap(),
        Data::Instance(Lookup {
            value: None,
            total: 0,
            complete: true,
            ..
        })
    ));
}
