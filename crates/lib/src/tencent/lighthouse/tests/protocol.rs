use super::*;
use crate::tencent::{api, model::Credential};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn receive(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut received = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        received.push_str(&line);
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap()
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    received.push_str(std::str::from_utf8(&body).unwrap());
    received
}

pub(super) fn exchange(request: Request, response: Value) -> Receipt {
    let call = request.call().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = receive(&stream);
        let body = json!({"Response": response}).to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        received
    });
    let credential = Credential {
        id: "AKID_FIXTURE".into(),
        key: "SECRET_FIXTURE".into(),
        token: Some("TOKEN_FIXTURE".into()),
        expires: None,
    };
    let body = api::Client::new(&credential)
        .request(
            (&origin, "lighthouse.tencentcloudapi.com"),
            api::Operation::new(call.action, "2020-03-24", Some(&request.region)),
            &call.body,
        )
        .unwrap();
    let result = request.receipt(identity(), &body.0).unwrap();

    let received = worker.join().unwrap();
    let header = received.to_ascii_lowercase();
    for part in [
        "host: lighthouse.tencentcloudapi.com",
        "x-tc-region: na-ashburn",
        "x-tc-version: 2020-03-24",
        "x-tc-token: token_fixture",
        "tc3-hmac-sha256 credential=akid_fixture/",
    ] {
        assert!(header.contains(part), "{part}");
    }
    assert!(header.contains(&format!(
        "x-tc-action: {}",
        call.action.to_ascii_lowercase()
    )));
    assert!(received.ends_with(&call.body.to_string()));
    result
}

#[test]
fn wire() {
    let receipt = exchange(
        super::instance(),
        fixture::page("InstanceSet", fixture::instance()),
    );
    assert_eq!(receipt.outcome, Outcome::Observed);
}

#[test]
fn shapes() {
    for (request, action, expected) in [
        (
            super::instance(),
            "DescribeInstances",
            json!({"InstanceIds":["lhins-fixture"],"Offset":0,"Limit":1}),
        ),
        (
            super::key(),
            "DescribeKeyPairs",
            json!({"KeyIds":["lhkp-fixture"],"Offset":0,"Limit":1}),
        ),
        (
            request(Operation::Keys(Window {
                offset: 100,
                limit: 100,
            })),
            "DescribeKeyPairs",
            json!({"Offset":100,"Limit":100}),
        ),
    ] {
        let call = request.call().unwrap();
        assert_eq!(call.action, action);
        assert_eq!(call.body, expected);
        assert_eq!(call.request(&request.region).service, "lighthouse");
        assert_eq!(call.request(&request.region).region, Some("na-ashburn"));
    }
    for operation in [
        Operation::Instance {
            id: "ins-wrong".into(),
        },
        Operation::Key {
            id: "skey-wrong".into(),
        },
        Operation::Keys(Window {
            offset: 0,
            limit: 0,
        }),
        Operation::Keys(Window {
            offset: 0,
            limit: 101,
        }),
    ] {
        assert_eq!(
            request(operation).validate().unwrap_err().class,
            Class::Input
        );
    }
    for region in ["na-ashburn-1", "", "NA-ASHBURN", "na-ashburn/path"] {
        let mut request = super::instance();
        request.region = region.into();
        assert!(request.validate().is_err());
    }
}

#[test]
fn receipt() {
    let body = fixture::page("KeyPairSet", fixture::key());
    let value = super::key().receipt(identity(), &body).unwrap();
    assert_eq!(value.outcome, Outcome::Observed);
    assert_eq!(value.service, "lighthouse");
    assert_eq!(value.version, "2020-03-24");
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let failure = super::key()
        .receipt(
            identity(),
            &json!({"TotalCount":1,"RequestId":"fixture-id"}),
        )
        .unwrap_err();
    assert_eq!(failure.request.as_deref(), Some("fixture-id"));
    let failure = super::key()
        .receipt(identity(), &json!({"RequestId":"PRIVATE MARKER"}))
        .unwrap_err();
    assert!(failure.request.is_none());
    assert!(!format!("{failure:?}").contains("PRIVATE MARKER"));
}

#[test]
fn command() {
    for (words, operation) in [
        (vec!["instance", "--id", "lhins-fixture"], super::instance()),
        (vec!["key", "--id", "lhkp-fixture"], super::key()),
        (
            vec!["keys", "--offset", "0", "--limit", "20"],
            super::keys(),
        ),
    ] {
        let words: Vec<_> = ["lighthouse", "--region", "na-ashburn"]
            .into_iter()
            .chain(words)
            .map(str::to_owned)
            .collect();
        let parsed = crate::tencent::parse(&words).unwrap();
        let crate::tencent::command::Deed::Lighthouse(deed) = parsed.deed else {
            panic!("grammar")
        };
        assert_eq!(
            deed.convert().unwrap().call().unwrap().body,
            operation.call().unwrap().body
        );
    }
    for verb in ["import", "associate", "delete", "create", "run-command"] {
        let words = ["lighthouse", "--region", "na-ashburn", verb].map(str::to_owned);
        assert!(crate::tencent::parse(&words).is_err());
    }
}
