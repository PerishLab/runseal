use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use serde_json::json;

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

#[test]
fn refused() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let body = json!({"fixture": "PRIVATE_BODY_MARKER"});
    for verb in ["GET", "DELETE"] {
        let error = runseal::http::Request::new(verb, &url)
            .json(&body)
            .send()
            .err()
            .expect("body must refuse before network access");
        assert_eq!(
            error.to_string(),
            format!("HTTP {verb} requests do not support a JSON body")
        );
        assert!(!format!("{error:?}").contains("PRIVATE_BODY_MARKER"));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn methods() {
    for verb in ["GET", "DELETE", "POST", "PATCH", "PUT"] {
        let (url, handle) = serve("200 OK", "{}", 1);
        let body = json!({"fixture": "public-marker"});
        let mut request = runseal::http::Request::new(verb, &url);
        let encoded = matches!(verb, "POST" | "PATCH" | "PUT");
        if encoded {
            request = request.json(&body);
        }
        assert_eq!(request.send().expect("response").status, 200);
        let wire = handle.join().expect("server");
        assert!(wire.starts_with(&format!("{verb} / HTTP/1.1\r\n")));
        let (_, received) = wire.split_once("\r\n\r\n").expect("headers");
        assert_eq!(
            received,
            if encoded {
                r#"{"fixture":"public-marker"}"#
            } else {
                ""
            }
        );
    }
}

fn serve(status: &str, body: &str, _: usize) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let status = status.to_string();
    let body = body.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = receive(&mut stream);
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nRetry-After: 7\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).expect("reply");
        request
    });
    (format!("http://{addr}"), handle)
}

fn receive(stream: &mut std::net::TcpStream) -> String {
    let mut request = Vec::new();
    loop {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).expect("request");
        assert!(count > 0, "request ended before its body");
        request.extend_from_slice(&chunk[..count]);
        let Some(at) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
            continue;
        };
        let length = String::from_utf8_lossy(&request[..at])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("length"))
            })
            .unwrap_or(0);
        if request.len() >= at + 4 + length {
            return String::from_utf8(request).expect("request text");
        }
    }
}
