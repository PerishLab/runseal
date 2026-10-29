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
