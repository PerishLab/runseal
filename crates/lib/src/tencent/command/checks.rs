use super::{Deed, Mode};
use crate::tencent::parse;
use serde_json::{Value, json};

#[test]
fn receipts() {
    let cases: &[(&[&str], Mode, Value)] = &[
        (
            &["cam", "user", "get", "--name", "fixture"],
            Mode::Read,
            json!({"Uin": 123}),
        ),
        (
            &["cam", "user", "create", "--name", "fixture"],
            Mode::Write,
            json!({"Uin": "123"}),
        ),
        (&["cam", "policy", "list"], Mode::Read, json!({"List": []})),
        (
            &["cam", "policy", "get", "--id", "1"],
            Mode::Read,
            json!({"PolicyDocument": "{}"}),
        ),
        (
            &["cam", "key", "list", "--user", "123"],
            Mode::Read,
            json!({"AccessKeys": []}),
        ),
        (
            &[
                "cam",
                "key",
                "create",
                "--user",
                "123",
                "--value-file",
                "/unused",
            ],
            Mode::Write,
            json!({"AccessKey": {"AccessKeyId":"fixture-key"}}),
        ),
    ];
    for (words, mode, held) in cases {
        let mut value = held.clone();
        value["RequestId"] = json!("fixture-id");
        let input = parse(
            &words
                .iter()
                .map(|word| (*word).to_string())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let bad = json!({"RequestId": "fixture", "Message": "PRIVATE_RESPONSE_MARKER"});
        let (request, malformed) = match &input.deed {
            Deed::Cam(deed) => {
                super::cam::receipt(deed, &value).unwrap();
                (
                    super::cam::request(deed).unwrap(),
                    super::cam::receipt(deed, &bad),
                )
            }
            _ => unreachable!(),
        };
        assert_eq!(request.mode, *mode);
        let error = malformed.unwrap_err();
        assert!(error.to_string().contains("may have completed"));
        assert!(!format!("{error:?}").contains("PRIVATE_RESPONSE_MARKER"));
    }
}

#[test]
fn compute() {
    let cases: &[(&[&str], &str, Value)] = &[
        (
            &[
                "compute",
                "instances",
                "--product",
                "lighthouse",
                "--region",
                "ap-hongkong",
            ],
            "DescribeInstances",
            json!({"Offset":0,"Limit":20}),
        ),
        (
            &[
                "compute",
                "bundles",
                "--region",
                "ap-hongkong",
                "--blueprint",
                "lhbp-fixture",
                "--offset",
                "40",
                "--limit",
                "10",
            ],
            "DescribeBundles",
            json!({"Offset":40,"Limit":10,"BlueprintId":"lhbp-fixture"}),
        ),
    ];
    for (words, action, body) in cases {
        let input = parse(
            &words
                .iter()
                .map(|word| (*word).to_string())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let Deed::Compute(query) = input.deed else {
            panic!("compute");
        };
        let query: crate::tencent::Query = query.into();
        let call = query.call().unwrap();
        assert_eq!(call.product, crate::tencent::Product::Lighthouse);
        assert_eq!(call.region.as_deref(), Some("ap-hongkong"));
        assert_eq!(call.action, *action);
        assert_eq!(call.body, *body);
    }
    let words = [
        "compute",
        "instances",
        "--product",
        "unknown",
        "--region",
        "ap-hongkong",
    ]
    .map(str::to_owned);
    assert!(parse(&words).is_err());
}

#[test]
fn enrollment() {
    use super::{Cam, Deed};
    let words = ["cam", "user", "create", "--name", "fixture"].map(String::from);
    let parsed = crate::tencent::parse(&words).unwrap();
    let Deed::Cam(deed) = parsed.deed else {
        panic!("CAM operation");
    };
    let request = super::cam::request(&deed).unwrap();
    assert_eq!(request.action, "AddUser");
    assert_eq!(request.mode, super::Mode::Write);
    assert_eq!(
        request.body,
        json!({"Name":"fixture","ConsoleLogin":0,"UseApi":0})
    );
    let words = ["cam", "key", "list", "--user", "0"].map(String::from);
    let parsed = crate::tencent::parse(&words).unwrap();
    let Deed::Cam(Cam::Key(deed)) = parsed.deed else {
        panic!("key operation");
    };
    assert!(super::cam::request(&Cam::Key(deed)).is_err());
}

#[test]
fn projection() {
    use super::model;
    use crate::tencent::api::project;
    let value = json!({
        "RequestId":"fixture-id", "PolicyId":99,
        "AccessKey":{"AccessKeyId":"public-id", "SecretAccessKey":"PRIVATE_MARKER", "PolicyId":33},
        "SecretKey":"PRIVATE_MARKER"
    });
    assert_eq!(
        project::<model::Issued>(&value).unwrap(),
        json!({
            "RequestId":"fixture-id", "AccessKey":{"AccessKeyId":"public-id"}
        })
    );
    assert_eq!(
        project::<model::Ack>(&value).unwrap(),
        json!({"RequestId":"fixture-id"})
    );
    assert_eq!(
        project::<model::Created>(&value).unwrap(),
        json!({"RequestId":"fixture-id", "PolicyId":99})
    );
    for value in [
        json!({"RequestId":"fixture-id"}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":""}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":{"SecretKey":"PRIVATE_MARKER"}}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":vec![json!({"AccessKeyId":"public-id"});201]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":"public-id", "Description":"x".repeat(128*1024+1)}]}),
    ] {
        let error = project::<model::Keys>(&value).unwrap_err();
        let fault = error.downcast_ref::<crate::tencent::Failure>().unwrap();
        assert_eq!(fault.request.as_deref(), Some("fixture-id"));
        assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    }
    for keys in [
        json!(null),
        json!([]),
        json!(vec![json!({"AccessKeyId":"public-id"}); 200]),
    ] {
        let output =
            project::<model::Keys>(&json!({"RequestId":"fixture-id", "AccessKeys":keys})).unwrap();
        assert_eq!(output["AccessKeys"], keys);
    }
    for id in [
        json!(""),
        json!("PRIVATE_MARKER\n"),
        json!(7),
        json!("x".repeat(129)),
    ] {
        let error = project::<model::Ack>(&json!({"RequestId":id})).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<crate::tencent::Failure>()
                .unwrap()
                .request,
            None
        );
        assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    }
    assert!(
        project::<model::User>(&json!({"RequestId":"fixture-id", "Uin":"not-number"})).is_err()
    );
    assert!(
        project::<model::Policies>(
            &json!({"RequestId":"fixture-id", "List":[{"PolicyId":1,"Type":"PRIVATE_MARKER"}]})
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn delivery() {
    use crate::tencent::{deliver, secret::Reserved};
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("issued.json");
    let body = json!({"RequestId":"fixture-id", "AccessKey":{"AccessKeyId":"public-id", "SecretAccessKey":"PRIVATE_MARKER", "PolicyId":99}});
    let result = crate::tencent::api::project::<super::model::Issued>(&body).unwrap();
    let receipt = deliver(Reserved::open(&path).unwrap(), &body, 123, result.clone()).unwrap();
    assert_eq!(receipt["result"], json!({"AccessKeyId":"public-id"}));
    assert_eq!(receipt["requestId"], "fixture-id");
    assert!(!format!("{receipt:?}").contains("PRIVATE_MARKER"));
    let bytes = crate::custody::Private::current()
        .read(&path, 16384)
        .unwrap();
    let state: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state["version"], 1);
    assert_eq!(state["credential"]["key"], "PRIVATE_MARKER");
    let refused = root.path().join("incomplete.json");
    let error = deliver(
        Reserved::open(&refused).unwrap(),
        &json!({"AccessKey":{"AccessKeyId":"public-id"}}),
        123,
        result,
    )
    .unwrap_err();
    assert!(error.to_string().contains("may have completed"));
    assert!(error.to_string().contains("fixture-id"));
    assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    assert!(!refused.exists());
}
