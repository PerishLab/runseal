use super::{
    api::{self, Client, Operation},
    model::Credential,
    sign,
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn credential() -> Credential {
    Credential {
        id: "AKID_EXAMPLE".into(),
        key: "SECRET_KEY_EXAMPLE".into(),
        token: Some("SESSION_FIXTURE".into()),
        expires: None,
    }
}

#[test]
fn reference() {
    let body = r#"{"Limit": 1, "Filters": [{"Values": ["unnamed"], "Name": "instance-name"}]}"#;
    let credential = credential();
    let actual = sign::authorization(
        &credential,
        ("cvm.tencentcloudapi.com", "cvm", "DescribeInstances"),
        body,
        1551113065,
    )
    .unwrap();
    assert_eq!(
        actual,
        "TC3-HMAC-SHA256 Credential=AKID_EXAMPLE/2019-02-25/cvm/tc3_request, SignedHeaders=content-type;host;x-tc-action, Signature=3288866c9d3b6f14353ce125ddc959997dc1971168f8e4fc72a05e328626ee67"
    );
    let other = sign::authorization(
        &credential,
        ("cvm.tencentcloudapi.com", "cvm", "DeleteInstances"),
        body,
        1551113065,
    )
    .unwrap();
    assert_ne!(actual, other);
    let changed = sign::authorization(
        &credential,
        ("cvm.tencentcloudapi.com", "cvm", "DescribeInstances"),
        "{}",
        1551113065,
    )
    .unwrap();
    assert_ne!(actual, changed);
}

#[test]
fn published() {
    let body =
        r#"{"Limit": 1, "Filters": [{"Values": ["\u672a\u547d\u540d"], "Name": "instance-name"}]}"#;
    assert_eq!(
        sign::hash(body.as_bytes()),
        "35e9c5b0e3ae67532d3c9f17ead6c90222632e5b1ff7f6e89887f1398934f064"
    );
    let mut credential = credential();
    credential.id = "AKID********************************".into();
    credential.key = "********************************".into();
    let actual = sign::authorization(
        &credential,
        ("cvm.tencentcloudapi.com", "cvm", "DescribeInstances"),
        body,
        1551113065,
    )
    .unwrap();
    assert_eq!(
        actual,
        "TC3-HMAC-SHA256 Credential=AKID********************************/2019-02-25/cvm/tc3_request, SignedHeaders=content-type;host;x-tc-action, Signature=10b1a37a7301a02ca19a647ad722d5e43b4b3cff309d421d85b46093f6ab6c4f"
    );
}

#[test]
fn wire() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = request(&mut stream);
        assert!(
            request.contains("X-TC-Token: SESSION_FIXTURE")
                || request.contains("x-tc-token: SESSION_FIXTURE")
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("x-tc-action: getuser")
        );
        let response = r#"{"Response":{"Name":"fixture","Uin":123,"RequestId":"fixture"}}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        assert!(request.ends_with(r#"{"Name":"fixture"}"#));
    });
    let credential = credential();
    let body = Client::new(&credential)
        .request(
            (&url, "cam.tencentcloudapi.com"),
            Operation::new("GetUser", "2019-01-16", Some("ap-guangzhou")),
            &json!({"Name":"fixture"}),
        )
        .unwrap();
    assert_eq!(body.0["Name"], "fixture");
    worker.join().unwrap();
}

#[test]
fn refusal() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = request(&mut stream);
        let body = r#"{"Response":{"Error":{"Code":"Denied","Message":"SECRET_RESPONSE_FIXTURE"},"SecretKey":"SECRET_RESPONSE_FIXTURE"}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let credential = credential();
    let result = Client::new(&credential).request(
        (&url, "cam.tencentcloudapi.com"),
        Operation::new("AddUser", "2019-01-16", Some("ap-guangzhou")),
        &json!({}),
    );
    let error = result.err().expect("provider refusal");
    assert!(!format!("{error:#?} {error:#}").contains("SECRET_RESPONSE_FIXTURE"));
    worker.join().unwrap();
}

#[test]
fn ambiguous() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = request(&mut stream);
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        for _ in 0..20 {
            assert!(listener.accept().is_err(), "mutation must not be replayed");
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });
    let credential = credential();
    let error = Client::new(&credential)
        .request(
            (&url, "cam.tencentcloudapi.com"),
            Operation::new("AddUser", "2019-01-16", Some("ap-guangzhou")),
            &json!({}),
        )
        .err()
        .expect("unknown result");
    assert!(error.to_string().contains("uncertain"));
    worker.join().unwrap();
}

#[test]
fn metadata() {
    let value = super::api::metadata(
        &json!({"Uin":123,"SecretKey":"SECRET","AccessKey":{"SecretAccessKey":"SECRET"},"AccessToken":"SECRET","RecordList":[{"RecordId":1,"SecretKey":"SECRET"}]}),
    );
    assert_eq!(value, json!({"Uin":123,"RecordList":[{"RecordId":1}]}));
    assert!(!value.to_string().contains("SECRET"));
    let _ = api::agent();
}

#[test]
fn incomplete() {
    for body in [
        r#"{"Response":{}}"#,
        r#"{"Response":{"RequestId":"fixture"}}"#,
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let credential = credential();
        let error = Client::new(&credential)
            .request(
                (&url, "cam.tencentcloudapi.com"),
                Operation::new("AddUser", "2019-01-16", Some("ap-guangzhou")),
                &json!({}),
            )
            .err()
            .expect("incomplete issuance must not claim success");
        assert!(error.to_string().contains("may have completed"));
        worker.join().unwrap();
    }
}

#[test]
fn oversized() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = request(&mut stream);
        let body = format!(
            "{{\"Response\":{{\"RequestId\":\"fixture\",\"Uin\":123,\"Name\":\"{}\"}}}}",
            "X".repeat(2 * 1024 * 1024)
        );
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    });
    let credential = credential();
    let error = Client::new(&credential)
        .request(
            (&url, "cam.tencentcloudapi.com"),
            Operation::new("GetUser", "2019-01-16", Some("ap-guangzhou")),
            &json!({}),
        )
        .err()
        .expect("oversized response must be refused");
    assert!(error.to_string().contains("response unreadable"));
    assert!(error.to_string().len() < 200);
    worker.join().unwrap();
}

fn request(stream: &mut std::net::TcpStream) -> String {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut room = [0; 1024];
    loop {
        let length = stream.read(&mut room).unwrap();
        assert!(length != 0);
        bytes.extend_from_slice(&room[..length]);
        assert!(bytes.len() < 16384);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let size = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if bytes.len() >= end + 4 + size {
                return String::from_utf8(bytes).unwrap();
            }
        }
    }
}

#[test]
fn enrollment() {
    use super::command::{Cam, Deed};
    let words = ["cam", "user", "create", "--name", "fixture"].map(String::from);
    let parsed = super::parse(&words).unwrap();
    let Deed::Cam(deed) = parsed.deed else {
        panic!("CAM operation");
    };
    let (action, value, mutation) = super::command::cam::request(&deed).unwrap();
    assert_eq!(action, "AddUser");
    assert!(mutation);
    assert_eq!(value, json!({"Name":"fixture","ConsoleLogin":0,"UseApi":0}));
    let words = ["cam", "key", "list", "--user", "0"].map(String::from);
    let parsed = super::parse(&words).unwrap();
    let Deed::Cam(Cam::Key(deed)) = parsed.deed else {
        panic!("key operation");
    };
    assert!(super::command::cam::request(&Cam::Key(deed)).is_err());
}
