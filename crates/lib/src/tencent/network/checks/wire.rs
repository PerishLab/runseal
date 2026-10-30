use super::{Error, Group, Operation, Outcome, Vpc, tests::request};
use crate::tencent::{Class, Failure, api, model::Credential};
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

pub(super) fn receive(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut header = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        header.push_str(&line);
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    header.push_str(std::str::from_utf8(&body).unwrap());
    header
}

pub(super) fn credential() -> Credential {
    Credential {
        id: "AKID_FIXTURE".into(),
        key: "SECRET_FIXTURE".into(),
        token: Some("TOKEN_FIXTURE".into()),
        expires: None,
    }
}

#[test]
fn signed() {
    let call = request(Operation::Vpc(Vpc::Create {
        name: "fixture".into(),
        cidr: "10.42.0.0/16".into(),
    }))
    .call()
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = receive(&stream);
        let body = r#"{"Response":{"RequestId":"fixture","Vpc":{"VpcId":"vpc-fixture","VpcName":"fixture","CidrBlock":"10.42.0.0/16","IsDefault":false,"Password":"SECRET_FIXTURE"}}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        received
    });
    let credential = credential();
    let body = api::Client::new(&credential)
        .request(
            (&origin, "vpc.tencentcloudapi.com"),
            api::Operation::new(
                call.action,
                api::version("vpc").unwrap(),
                Some("ap-hongkong"),
            ),
            &call.body,
        )
        .unwrap();
    let data = call.decode(&body.0).unwrap();
    assert!(
        !serde_json::to_string(&data)
            .unwrap()
            .contains("SECRET_FIXTURE")
    );
    let received = worker.join().unwrap();
    let header = received.to_ascii_lowercase();
    assert!(header.contains("host: vpc.tencentcloudapi.com"));
    assert!(header.contains("x-tc-action: createvpc"));
    assert!(header.contains("x-tc-version: 2017-03-12"));
    assert!(header.contains("x-tc-region: ap-hongkong"));
    assert!(header.contains("x-tc-token: token_fixture"));
    assert!(received.ends_with(&call.body.to_string()));
    assert!(call.body.get("ClientToken").is_none());
}

#[test]
fn uncertainty() {
    for (body, expected) in [
        (None, Outcome::Unknown),
        (
            Some(
                r#"{"Response":{"Error":{"Code":"UnsupportedOperation.VersionMismatch","Message":"SECRET_FIXTURE"},"RequestId":"fixture"}}"#,
            ),
            Outcome::Rejected,
        ),
        (
            Some(
                r#"{"Response":{"Error":{"Code":"UNKNOWN_SECRET_FIXTURE","Message":"SECRET_FIXTURE"},"RequestId":"fixture"}}"#,
            ),
            Outcome::Unknown,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            receive(&stream);
            if let Some(body) = body {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
            drop(stream);
            attempts(&listener)
        });
        let credential = credential();
        let fault = api::Client::new(&credential)
            .request(
                (&origin, "vpc.tencentcloudapi.com"),
                api::Operation::new("DeleteVpc", "2017-03-12", Some("ap-hongkong")),
                &json!({"VpcId":"vpc-fixture"}),
            )
            .err()
            .expect("refusal");
        let error = Error::remote(Failure::capture(fault, Class::Response));
        assert_eq!(format!("{:?}", error.outcome), format!("{expected:?}"));
        assert!(!format!("{error:?} {error}").contains("SECRET_FIXTURE"));
        assert_eq!(worker.join().unwrap(), 1);
    }
}

#[test]
fn admission() {
    let query = request(Operation::Group(Group::Reset {
        id: "sg-fixture".into(),
        rules: super::Rules {
            version: 1,
            ingress: vec![],
            egress: vec![],
        },
    }));
    let call = query.call().unwrap();
    assert!(call.mutation());
    assert!(matches!(
        call.decode(&json!({"RequestId":"fixture"})).unwrap(),
        super::Data::Ack
    ));
    assert_eq!(call.body["SecurityGroupPolicySet"]["Ingress"], json!([]));
    assert_eq!(call.body["SecurityGroupPolicySet"]["Egress"], json!([]));
}

fn attempts(listener: &TcpListener) -> usize {
    listener.set_nonblocking(true).unwrap();
    let mut attempts = 1;
    for _ in 0..20 {
        if listener.accept().is_ok() {
            attempts += 1;
        }
        thread::sleep(std::time::Duration::from_millis(5));
    }
    attempts
}
