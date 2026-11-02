use super::*;
use crate::tencent::{api, model::Credential};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

pub(super) fn receive(stream: &TcpStream) -> String {
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

pub(super) fn credential() -> Credential {
    Credential {
        id: "AKID_FIXTURE".into(),
        key: "SECRET_FIXTURE".into(),
        token: Some("TOKEN_FIXTURE".into()),
        expires: None,
    }
}

#[test]
fn protocol() {
    let request = agent();
    let call = request.call().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = receive(&stream);
        let body = r#"{"Response":{"RequestId":"fixture-id","AutomationAgentSet":[],"TotalCount":0,"Password":"PRIVATE_MARKER"}}"#;
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
            (&origin, "tat.tencentcloudapi.com"),
            api::Operation::new(call.action, "2020-10-28", Some(&request.region)),
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
    assert!(header.contains("host: tat.tencentcloudapi.com"));
    assert!(header.contains("x-tc-region: na-ashburn"));
    assert!(header.contains("x-tc-action: describeautomationagentstatus"));
    assert!(header.contains("x-tc-version: 2020-10-28"));
    assert!(header.contains("x-tc-token: token_fixture"));
    assert!(header.contains("tc3-hmac-sha256 credential=akid_fixture/"));
    assert!(received.ends_with(&call.body.to_string()));
}
