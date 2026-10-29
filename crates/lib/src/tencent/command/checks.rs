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
            json!({"AccessKey": {}}),
        ),
    ];
    for (words, mode, value) in cases {
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
                super::cam::receipt(deed, value).unwrap();
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
