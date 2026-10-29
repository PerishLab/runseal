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
        (
            &["dns", "domain", "--domain", "example.com"],
            Mode::Read,
            json!({"DomainInfo": {}}),
        ),
        (
            &["dns", "record", "list", "--domain", "example.com"],
            Mode::Read,
            json!({"RecordList": []}),
        ),
        (
            &[
                "dns",
                "record",
                "create",
                "--domain",
                "example.com",
                "--name",
                "fixture",
                "--kind",
                "A",
                "--value",
                "192.0.2.1",
            ],
            Mode::Write,
            json!({"RecordId": 1}),
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
            Deed::Dns(deed) => {
                super::dns::receipt(deed, value).unwrap();
                (
                    super::dns::request(deed).unwrap(),
                    super::dns::receipt(deed, &bad),
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
