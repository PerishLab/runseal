use super::*;
use base64::Engine;

pub(super) fn import() -> Request {
    let mut bytes = Vec::new();
    let mut modulus = vec![0u8; 257];
    modulus[1] = 0x80;
    modulus[256] = 1;
    for field in [b"ssh-rsa".as_slice(), &[1, 0, 1], &modulus] {
        bytes.extend_from_slice(&(field.len() as u32).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    request(Operation::Import(Import {
        name: "fixture".into(),
        public: format!(
            "ssh-rsa {} fixture",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ),
    }))
}

pub(super) fn associate() -> Request {
    request(Operation::Associate(Binding {
        key: "lhkp-fixture".into(),
        instance: "lhins-fixture".into(),
        username: "ubuntu".into(),
    }))
}

pub(super) fn delete() -> Request {
    request(Operation::Delete {
        id: "lhkp-fixture".into(),
    })
}

#[test]
fn dispatch() {
    let imported = import();
    let Operation::Import(public) = &imported.operation else {
        panic!()
    };
    let call = imported.call().unwrap();
    assert_eq!(call.action, "ImportKeyPair");
    assert_eq!(
        call.body,
        json!({"KeyName":"fixture","PublicKey":public.public})
    );
    let call = associate().call().unwrap();
    assert_eq!(call.action, "AssociateInstancesKeyPairs");
    assert_eq!(
        call.body,
        json!({"KeyIds":["lhkp-fixture"],"InstanceIds":["lhins-fixture"],"AssociateType":"ONLINE","Username":"ubuntu"})
    );
    let call = delete().call().unwrap();
    assert_eq!(call.action, "DeleteKeyPairs");
    assert_eq!(call.body, json!({"KeyIds":["lhkp-fixture"]}));
    for request in [imported, associate(), delete()] {
        assert_eq!(
            request.call().unwrap().request(&request.region).region,
            Some("na-ashburn")
        );
    }
}

#[test]
fn acceptance() {
    let response = json!({"KeyId":"lhkp-returned","RequestId":"fixture-id"});
    let receipt = import().receipt(identity(), &response).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    let Data::Imported(value) = receipt.data else {
        panic!()
    };
    assert_eq!(value.id, "lhkp-returned");
    assert!(value.fingerprint.starts_with("SHA256:"));
    assert!(value.complete);
    let mut imported = import();
    let Operation::Import(public) = &mut imported.operation else {
        panic!()
    };
    public.public = public.public.replace("fixture", "different-comment");
    let Data::Imported(other) = imported.decode(&response).unwrap() else {
        panic!()
    };
    assert_eq!(value.fingerprint, other.fingerprint);
    for request in [associate(), delete()] {
        let receipt = request
            .receipt(identity(), &json!({"RequestId":"fixture-id"}))
            .unwrap();
        assert_eq!(receipt.outcome, Outcome::Accepted);
        let value = serde_json::to_value(receipt.data).unwrap();
        assert_eq!(value["complete"], true);
        assert!(value.to_string().contains("lhkp-fixture"));
        let receipt = request
            .receipt(
                identity(),
                &json!({"RequestId":"fixture-id","Unexpected":"PRIVATE_MARKER"}),
            )
            .unwrap();
        let value = serde_json::to_value(receipt.data).unwrap();
        assert_eq!(value["complete"], false);
        assert!(!value.to_string().contains("PRIVATE_MARKER"));
    }
    for response in [
        json!({"RequestId":"fixture-id"}),
        json!({"KeyId":"skey-foreign","RequestId":"fixture-id"}),
    ] {
        let failure = import().receipt(identity(), &response).unwrap_err();
        assert_eq!(failure.request.as_deref(), Some("fixture-id"));
    }
    for request in [import(), associate(), delete()] {
        assert!(request.receipt(identity(), &json!({})).is_err());
        assert!(
            request
                .receipt(identity(), &json!({"RequestId":"PRIVATE MARKER"}))
                .is_err()
        );
    }
}

#[test]
fn inputs() {
    for username in [
        "",
        "-root",
        "Root",
        "root\n",
        "root;id",
        "a/b",
        "1root",
        "root name",
    ] {
        let mut request = associate();
        let Operation::Associate(value) = &mut request.operation else {
            panic!()
        };
        value.username = username.into();
        assert_eq!(request.validate().unwrap_err().class, Class::Input);
    }
    for name in [
        "",
        "with-hyphen",
        "name path",
        "this_name_is_longer_than_25_bytes",
    ] {
        let mut request = import();
        let Operation::Import(value) = &mut request.operation else {
            panic!()
        };
        value.name = name.into();
        assert!(request.validate().is_err());
    }
    for public in [
        "PRIVATE_MARKER",
        "ssh-rsa AAAA",
        "ssh-ed25519 AAAA",
        "ssh-rsa AAAA\n",
    ] {
        let mut request = import();
        let Operation::Import(value) = &mut request.operation else {
            panic!()
        };
        value.public = public.into();
        let failure = request.validate().unwrap_err();
        assert!(!format!("{failure:?}").contains("PRIVATE_MARKER"));
    }
    let mut request = associate();
    let Operation::Associate(value) = &mut request.operation else {
        panic!()
    };
    value.key = "skey-foreign".into();
    assert!(request.validate().is_err());
    let mut request = associate();
    let Operation::Associate(value) = &mut request.operation else {
        panic!()
    };
    value.instance = "ins-foreign".into();
    assert!(request.validate().is_err());
    assert!(
        super::request(Operation::Delete {
            id: "skey-foreign".into()
        })
        .validate()
        .is_err()
    );
}

#[test]
fn command() {
    for (words, expected) in [
        (
            vec![
                "associate",
                "--key",
                "lhkp-fixture",
                "--instance",
                "lhins-fixture",
                "--username",
                "ubuntu",
            ],
            associate(),
        ),
        (vec!["delete", "--id", "lhkp-fixture"], delete()),
    ] {
        let words: Vec<_> = ["lighthouse", "--region", "na-ashburn"]
            .into_iter()
            .chain(words)
            .map(str::to_owned)
            .collect();
        let parsed = crate::tencent::parse(&words).unwrap();
        let crate::tencent::command::Deed::Lighthouse(deed) = parsed.deed else {
            panic!()
        };
        assert_eq!(
            deed.convert().unwrap().call().unwrap().body,
            expected.call().unwrap().body
        );
    }
    for suffix in [
        vec!["--offline"],
        vec!["--username", "root", "--type", "OFFLINE"],
        vec![],
    ] {
        let words: Vec<_> = [
            "lighthouse",
            "--region",
            "na-ashburn",
            "associate",
            "--key",
            "lhkp-fixture",
            "--instance",
            "lhins-fixture",
        ]
        .into_iter()
        .chain(suffix)
        .map(str::to_owned)
        .collect();
        assert!(crate::tencent::parse(&words).is_err());
    }
}

#[test]
fn wire() {
    for request in [import(), associate(), delete()] {
        let body = if matches!(request.operation, Operation::Import(_)) {
            json!({"KeyId":"lhkp-returned","RequestId":"fixture-id"})
        } else {
            json!({"RequestId":"fixture-id"})
        };
        let receipt = super::protocol::exchange(request, body);
        assert_eq!(receipt.outcome, Outcome::Accepted);
    }
}
