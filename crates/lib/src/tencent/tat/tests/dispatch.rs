use super::{
    execution::{query, script},
    wire::{credential, receive},
    *,
};
use crate::tencent::api;
use std::{io::Write, net::TcpListener, thread, time::Duration};

#[test]
fn protocol() {
    for operation in [Operation::Run(script()), Operation::Task(query(false))] {
        let input = request(operation);
        let call = input.call().unwrap();
        let reply = if call.action == "RunCommand" {
            json!({"CommandId":"cmd-fixture","InvocationId":"inv-fixture","RequestId":"fixture-id"})
        } else {
            execution::page(execution::row())
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let received = receive(&stream);
            let body = json!({"Response":reply}).to_string();
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
                api::Operation::new(call.action, "2020-10-28", Some("na-ashburn")),
                &call.body,
            )
            .unwrap();
        input.decode(&body.0).unwrap();
        let received = worker.join().unwrap();
        let headers = received.to_ascii_lowercase();
        assert!(headers.contains("host: tat.tencentcloudapi.com"));
        assert!(headers.contains("x-tc-version: 2020-10-28"));
        assert!(headers.contains("x-tc-region: na-ashburn"));
        assert!(headers.contains(&format!(
            "x-tc-action: {}",
            call.action.to_ascii_lowercase()
        )));
        assert!(headers.contains("tc3-hmac-sha256 credential=akid_fixture/"));
        let native: Value =
            serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        if call.action == "RunCommand" {
            assert_eq!(native["SaveCommand"], false);
            assert_eq!(native["EnableParameter"], false);
            assert_eq!(native["CommandType"], "SHELL");
            assert_eq!(native["Content"], "cHJpbnRmIHB1YmxpYy1ub25jZQo=");
            assert!(native.get("ClientToken").is_none());
            assert!(native.get("Parameters").is_none());
        } else {
            assert_eq!(native["HideOutput"], true);
        }
    }
}

#[test]
fn uncertainty() {
    for (body, expected) in [
        (None, Outcome::Unknown),
        (Some("<html>PRIVATE_MARKER</html>"), Outcome::Unknown),
        (
            Some(
                r#"{"Response":{"RequestId":"fixture-id","Error":{"Code":"UnauthorizedOperation","Message":"PRIVATE_MARKER"}}}"#,
            ),
            Outcome::Rejected,
        ),
        (
            Some(
                r#"{"Response":{"RequestId":"fixture-id","Error":{"Code":"PRIVATE_MARKER","Message":"PRIVATE_MARKER"}}}"#,
            ),
            Outcome::Unknown,
        ),
        (
            Some(r#"{"Response":{"RequestId":"fixture-id","InvocationId":"inv-fixture"}}"#),
            Outcome::Unknown,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
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
        let input = request(Operation::Run(script()));
        let call = input.call().unwrap();
        let observed = api::Client::new(&credential)
            .request(
                (&origin, "tat.tencentcloudapi.com"),
                api::Operation::new(call.action, "2020-10-28", Some("na-ashburn")),
                &call.body,
            )
            .map_err(|error| Error::remote(Failure::capture(error, Class::Response)))
            .and_then(|body| {
                input.decode(&body.0).map_err(|fault| Error {
                    fault,
                    outcome: Outcome::Unknown,
                })
            });
        let error = observed.unwrap_err();
        assert_eq!(error.outcome, expected);
        assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
        assert_eq!(worker.join().unwrap(), 1);
    }
}

fn attempts(listener: &TcpListener) -> usize {
    listener.set_nonblocking(true).unwrap();
    let mut attempts = 1;
    for _ in 0..20 {
        if listener.accept().is_ok() {
            attempts += 1;
        }
        thread::sleep(Duration::from_millis(5));
    }
    attempts
}
