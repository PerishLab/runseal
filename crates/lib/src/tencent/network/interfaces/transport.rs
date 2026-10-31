use super::*;
use crate::tencent::{api, network::wire};
use std::{
    io::{ErrorKind, Write},
    net::TcpListener,
    thread,
};

#[test]
fn signed() {
    let call = call(
        "ins-fixture",
        Window {
            offset: 0,
            limit: 10,
        },
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = wire::receive(&stream);
        let body = r#"{"Response":{"RequestId":"interfaces-fixture","TotalCount":0,"NetworkInterfaceSet":[]}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        drop(stream);
        (listener, received)
    });
    let credential = wire::credential();
    let body = api::Client::new(&credential)
        .request(
            (&origin, "vpc.tencentcloudapi.com"),
            api::Operation::new(call.action, "2017-03-12", Some("ap-hongkong")),
            &call.body,
        )
        .unwrap();
    let (listener, received) = worker.join().unwrap();
    listener.set_nonblocking(true).unwrap();
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    let lower = received.to_ascii_lowercase();
    for expected in [
        "host: vpc.tencentcloudapi.com",
        "x-tc-action: describenetworkinterfaces",
        "x-tc-region: ap-hongkong",
        "x-tc-version: 2017-03-12",
        "authorization: tc3-hmac-sha256",
        "x-tc-token: token_fixture",
    ] {
        assert!(lower.contains(expected));
    }
    assert!(received.ends_with(&call.body.to_string()));
    let Data::Interfaces { page } = call.decode(&body.0).unwrap() else {
        panic!("interfaces")
    };
    assert_eq!(page.total, 0);
    assert!(page.items.is_empty());
    assert_eq!(page.next, None);
}
