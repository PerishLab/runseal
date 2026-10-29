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
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    received.push_str(std::str::from_utf8(&body).unwrap());
    received
}

fn credential() -> Credential {
    Credential {
        id: "AKID_FIXTURE".into(),
        key: "SECRET_FIXTURE".into(),
        token: Some("TOKEN_FIXTURE".into()),
        expires: None,
    }
}

#[test]
fn protocol() {
    let request = request(Operation::Run {
        spec: spec(),
        token: "durable-intent".into(),
        dry: false,
    });
    let call = request.call().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = receive(&stream);
        let body = r#"{"Response":{"RequestId":"fixture-id","InstanceIdSet":["ins-created"],"Password":"PRIVATE_MARKER"}}"#;
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
            (&origin, "cvm.tencentcloudapi.com"),
            api::Operation::new(call.action, "2017-03-12", Some(&request.region)),
            &call.body,
        )
        .unwrap();
    let data = request.decode(&body.0).unwrap();
    assert!(
        !serde_json::to_string(&data)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let received = worker.join().unwrap();
    let header = received.to_ascii_lowercase();
    assert!(header.contains("host: cvm.tencentcloudapi.com"));
    assert!(header.contains("x-tc-region: ap-hongkong"));
    assert!(header.contains("x-tc-action: runinstances"));
    assert!(header.contains("x-tc-version: 2017-03-12"));
    assert!(header.contains("x-tc-token: token_fixture"));
    assert!(header.contains("tc3-hmac-sha256 credential=akid_fixture/"));
    assert!(received.ends_with(&call.body.to_string()));
}

#[test]
fn uncertainty() {
    for (body, expected) in [
        (None, Outcome::Unknown),
        (Some("<html>PRIVATE_MARKER</html>"), Outcome::Unknown),
        (
            Some(
                r#"{"Response":{"RequestId":"fixture-id","Error":{"Code":"LimitExceeded.CvmInstanceQuota","Message":"PRIVATE_MARKER"}}}"#,
            ),
            Outcome::Rejected,
        ),
        (
            Some(
                r#"{"Response":{"RequestId":"fixture-id","Error":{"Code":"UNKNOWN_PRIVATE_MARKER","Message":"PRIVATE_MARKER"}}}"#,
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
        let call = request(Operation::Terminate {
            id: "ins-fixture".into(),
        })
        .call()
        .unwrap();
        let fault = api::Client::new(&credential)
            .request(
                (&origin, "cvm.tencentcloudapi.com"),
                api::Operation::new(call.action, "2017-03-12", Some("ap-hongkong")),
                &call.body,
            )
            .err()
            .expect("refusal");
        let error = Error::remote(Failure::capture(fault, Class::Response));
        assert_eq!(error.outcome, expected);
        assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
        assert_eq!(worker.join().unwrap(), 1);
    }
}

fn attempts(listener: &TcpListener) -> usize {
    listener.set_nonblocking(true).unwrap();
    let mut count = 1;
    for _ in 0..20 {
        if listener.accept().is_ok() {
            count += 1;
        }
        thread::sleep(std::time::Duration::from_millis(5));
    }
    count
}
