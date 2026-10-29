use super::*;
use serde_json::json;

#[test]
fn purchase() {
    let checked = request(Operation::Run {
        spec: spec(),
        token: "durable-intent-1".into(),
        dry: true,
    });
    let call = checked.call().unwrap();
    assert_eq!(call.action, "RunInstances");
    let body = &call.body;
    assert_eq!(body["InstanceChargeType"], "POSTPAID_BY_HOUR");
    assert_eq!(body["InstanceCount"], 1);
    assert_eq!(body["ClientToken"], "durable-intent-1");
    assert_eq!(body["DryRun"], true);
    assert_eq!(body["DataDisks"], json!([]));
    assert_eq!(body["LoginSettings"]["KeyIds"], json!(["skey-fixture"]));
    assert_eq!(
        body["VirtualPrivateCloud"],
        json!({"VpcId":"vpc-fixture","SubnetId":"subnet-fixture"})
    );
    assert_eq!(body["SecurityGroupIds"], json!(["sg-fixture"]));
    assert_eq!(
        body["SystemDisk"],
        json!({"DiskType":"CLOUD_PREMIUM","DiskSize":50})
    );
    assert!(body.get("UserData").is_none());
    assert!(body.get("Password").is_none());
    assert!(body["LoginSettings"].get("Password").is_none());
    assert!(body["LoginSettings"].get("KeepImageLogin").is_none());
    let quote = request(Operation::Quote(spec())).call().unwrap();
    assert_eq!(quote.action, "InquiryPriceRunInstances");
    assert!(quote.body.get("DryRun").is_none());
    assert!(quote.body.get("ClientToken").is_none());
    assert!(!checked.mutation());
    assert!(
        request(Operation::Run {
            spec: spec(),
            token: "intent".into(),
            dry: false
        })
        .mutation()
    );
    assert!(matches!(checked.decode(&json!({})).unwrap(), Data::Checked));
    assert!(
        checked
            .decode(&json!({"InstanceIdSet":["ins-created"]}))
            .is_err()
    );
    let run = request(Operation::Run {
        spec: spec(),
        token: "intent".into(),
        dry: false,
    });
    for invalid in [
        json!({}),
        json!({"InstanceIdSet":[]}),
        json!({"InstanceIdSet":["ins-one","ins-two"]}),
    ] {
        assert!(run.decode(&invalid).is_err());
    }
    assert!(matches!(
        run.decode(&json!({"InstanceIdSet":["ins-one"]})).unwrap(),
        Data::Created(_)
    ));
}

#[test]
fn bounds() {
    for token in ["".into(), "x".repeat(65), "中文".into(), "unsafe\n".into()] {
        assert!(
            request(Operation::Run {
                spec: spec(),
                token,
                dry: false
            })
            .validate()
            .is_err()
        );
    }
    assert!(
        request(Operation::Run {
            spec: spec(),
            token: "x".repeat(64),
            dry: true
        })
        .validate()
        .is_ok()
    );
    let mut variants = Vec::new();
    let mut cross = spec();
    cross.placement.zone = "ap-shanghai-2".into();
    variants.push(cross);
    let mut foreign = spec();
    foreign.placement.vpc = "ins-wrong".into();
    variants.push(foreign);
    let mut groups = spec();
    groups.connection.groups.push("sg-fixture".into());
    variants.push(groups);
    let mut image = spec();
    image.hardware.image = "lh-image".into();
    variants.push(image);
    let mut disk = spec();
    disk.hardware.disk.size = 0;
    variants.push(disk);
    let mut public = spec();
    public.connection.public = false;
    variants.push(public);
    let mut key = spec();
    key.connection.key = "PRIVATE_MARKER".into();
    variants.push(key);
    for spec in variants {
        let failure = request(Operation::Quote(spec)).validate().unwrap_err();
        assert_eq!(failure.class, Class::Input);
        assert!(!format!("{failure:?}").contains("PRIVATE_MARKER"));
    }
    let terminate = request(Operation::Terminate {
        id: "ins-fixture".into(),
    })
    .call()
    .unwrap();
    assert_eq!(
        terminate.body,
        json!({"InstanceIds":["ins-fixture"],"ReleaseAddress":false,"ReleasePrepaidDataDisks":false})
    );
}

#[test]
fn keys() {
    let import = Import {
        name: "fixture_key".into(),
        project: 0,
        public: rsa(),
    };
    assert!(import.validate().is_ok());
    for bits in [2048, 4096] {
        assert!(
            Import {
                public: super::sized(bits),
                ..import.clone()
            }
            .validate()
            .is_ok()
        );
    }
    for bits in [1024, 8192] {
        assert!(
            Import {
                public: super::sized(bits),
                ..import.clone()
            }
            .validate()
            .is_err()
        );
    }
    let call = request(Operation::Import(import.clone())).call().unwrap();
    assert_eq!(call.action, "ImportKeyPair");
    assert!(call.body.get("PrivateKey").is_none());
    for public in [
        "-----BEGIN PRIVATE KEY-----".into(),
        "ssh-ed25519 AAAA fixture".into(),
        "ssh-rsa AAAA fixture".into(),
        "x".repeat(8193),
    ] {
        let error = Import {
            public,
            ..import.clone()
        }
        .validate()
        .unwrap_err();
        assert_eq!(error.class, Class::Input);
        assert!(!format!("{error:?}").contains("BEGIN"));
    }
    for name in ["with-hyphen".into(), "x".repeat(26)] {
        assert!(
            Import {
                name,
                ..import.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert_eq!(
        request(Operation::Delete {
            id: "skey-fixture".into()
        })
        .call()
        .unwrap()
        .body,
        json!({"KeyIds":["skey-fixture"]})
    );
}

#[test]
fn cli() {
    use crate::tencent::{command::Deed, parse};
    let args = [
        "cvm",
        "--region",
        "ap-hongkong",
        "status",
        "--id",
        "ins-fixture",
    ]
    .map(str::to_owned);
    let input = parse(&args).unwrap();
    let Deed::Cvm(deed) = input.deed else {
        panic!("cvm");
    };
    let converted = crate::tencent::command::cvm::request(&deed).unwrap();
    assert_eq!(
        converted.call().unwrap().body,
        request(Operation::Status {
            id: "ins-fixture".into()
        })
        .call()
        .unwrap()
        .body
    );
    for escape in [
        ["--count", "2"],
        ["--charge", "PREPAID"],
        ["--raw-body", "{}"],
        ["--endpoint", "https://invalid"],
    ] {
        let mut words = args.to_vec();
        words.extend(escape.map(str::to_owned));
        assert!(parse(&words).is_err());
    }
}
