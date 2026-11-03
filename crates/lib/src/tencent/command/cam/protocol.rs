use super::*;
use crate::tencent::{Error, Outcome, api, model::Credential};
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

fn exchange(request: Request, response: Option<Value>) -> Result<Receipt, Error> {
    let params = request.params().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = receive(&stream);
        if let Some(response) = response {
            let body = json!({"Response":response}).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        received
    });
    let credential = Credential {
        id: "AKID_FIXTURE".into(),
        key: "PRIVATE_KEY_FIXTURE".into(),
        token: Some("TOKEN_FIXTURE".into()),
        expires: None,
    };
    let body = api::Client::new(&credential)
        .request(
            (&origin, "cam.tencentcloudapi.com"),
            api::Operation::new(request.action(), "2019-01-16", None),
            &params,
        )
        .map_err(|error| {
            Error::remote(crate::tencent::Failure::capture(
                error,
                crate::tencent::Class::Response,
            ))
        });
    let result = body.and_then(|body| {
        record::receipt(&request, checks::identity(), &body.0).map_err(|fault| Error {
            fault,
            outcome: Outcome::Unknown,
        })
    });
    let received = worker.join().unwrap();
    let headers = received.to_ascii_lowercase();
    assert!(headers.contains("host: cam.tencentcloudapi.com"));
    assert!(headers.contains("x-tc-version: 2019-01-16"));
    assert!(headers.contains("x-tc-token: token_fixture"));
    assert!(headers.contains("tc3-hmac-sha256 credential=akid_fixture/"));
    assert!(!headers.contains("x-tc-region:"));
    assert!(headers.contains(&format!(
        "x-tc-action: {}",
        request.action().to_ascii_lowercase()
    )));
    assert!(received.ends_with(&params.to_string()));
    result
}

#[test]
fn wire() {
    let result = exchange(
        Request::Entities {
            policy: 42,
            page: 1,
        },
        Some(json!({"RequestId":"fixture-id","TotalNum":0,"List":[]})),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Observed);
    assert_eq!(result.service, "cam");
    let result = exchange(
        Request::Delete { policy: 42 },
        Some(json!({"RequestId":"fixture-id"})),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Accepted);
    assert_eq!(result.request, "fixture-id");
}

#[test]
fn uncertainty() {
    for (response, expected, code) in [
        (None, Outcome::Unknown, None),
        (Some(json!({})), Outcome::Unknown, None),
        (
            Some(
                json!({"RequestId":"fixture-id","Error":{"Code":"InvalidParameter.PolicyIdNotExist","Message":"PRIVATE_MARKER"}}),
            ),
            Outcome::Rejected,
            Some("InvalidParameter.PolicyIdNotExist"),
        ),
        (
            Some(
                json!({"RequestId":"fixture-id","Error":{"Code":"FailedOperation.UnknownPolicyFailure","Message":"PRIVATE_MARKER"}}),
            ),
            Outcome::Unknown,
            None,
        ),
    ] {
        let error = exchange(Request::Delete { policy: 42 }, response).unwrap_err();
        assert_eq!(error.outcome, expected);
        assert_eq!(error.fault.code.as_deref(), code);
        assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
    }
}
