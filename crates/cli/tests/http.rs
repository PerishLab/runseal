use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn fault() {
    let (url, handle) = serve("429 Too Many Requests", r#"{"message":"slow down"}"#, 1);
    let error = runseal::http::send("GET", &url, None, None).expect_err("request should fail");
    handle.join().expect("server");
    let fault = error
        .downcast_ref::<runseal::http::Fault>()
        .expect("structured fault");
    assert_eq!(fault.status(), 429);
    assert_eq!(fault.header("retry-after"), Some("7"));
    assert_eq!(fault.body()["message"], "slow down");
    assert_eq!(error.to_string(), "slow down");
    assert!(!format!("{fault:?}").contains("slow down"));
}

#[test]
fn response() {
    let (url, handle) = serve("404 Not Found", "missing", 1);
    let response = runseal::http::request("GET", &url, None, None).expect("response");
    handle.join().expect("server");
    assert_eq!(response.status, 404);
    assert_eq!(response.body, "missing");
}

fn serve(status: &str, body: &str, _: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let status = status.to_string();
    let body = body.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = [0u8; 1024];
        let mut read = 0usize;
        while read < request.len() {
            let count = stream.read(&mut request[read..]).expect("request");
            if count == 0 {
                break;
            }
            read += count;
            if request[..read].windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nRetry-After: 7\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).expect("reply");
    });
    (format!("http://{addr}"), handle)
}
