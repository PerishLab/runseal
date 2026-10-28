use super::super::{Class, api, model::Credential};
use super::{Product, Query, Window};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn receive(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        request.push_str(&line);
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    request.push_str(std::str::from_utf8(&body).unwrap());
    request
}

#[test]
fn wire() {
    for query in [
        Query::Instances {
            product: Product::Cvm,
            region: "ap-hongkong".into(),
            window: Window {
                offset: 20,
                limit: 100,
            },
        },
        Query::Regions {
            product: Product::Lighthouse,
        },
    ] {
        let call = query.call().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let response = if call.service == "cvm" {
            r#"{"Response":{"RequestId":"fixture","TotalCount":0,"InstanceSet":[]}}"#
        } else {
            r#"{"Response":{"RequestId":"fixture","TotalCount":0,"RegionSet":[]}}"#
        };
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let request = receive(&stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                response.len()
            )
            .unwrap();
            request
        });
        let credential = Credential {
            id: "AKID_FIXTURE".into(),
            key: "SECRET_FIXTURE".into(),
            token: Some("TOKEN_FIXTURE".into()),
            expires: None,
        };
        let host = format!("{}.tencentcloudapi.com", call.service);
        api::Client::new(&credential)
            .request(
                (&origin, &host),
                api::Operation::new(
                    call.action,
                    api::version(call.service).unwrap(),
                    call.region.as_deref(),
                ),
                &call.body,
            )
            .unwrap();
        let request = worker.join().unwrap().to_ascii_lowercase();
        assert!(request.contains(&format!("host: {host}")));
        assert!(request.contains(&format!(
            "x-tc-action: {}",
            call.action.to_ascii_lowercase()
        )));
        assert!(request.contains(&format!(
            "x-tc-version: {}",
            api::version(call.service).unwrap()
        )));
        if let Some(region) = &call.region {
            assert!(request.contains(&format!("x-tc-region: {region}")));
        } else {
            assert!(!request.contains("x-tc-region:"));
            assert_eq!(call.product, Product::Lighthouse);
        }
        assert!(request.contains("x-tc-token: token_fixture"));
        assert!(request.ends_with(&call.body.to_string().to_ascii_lowercase()));
    }
}

#[test]
fn refuses() {
    for region in [
        "",
        "-",
        "ap--hongkong",
        "AP-Hongkong",
        "https://invalid",
        "ap-hongkong\n",
    ] {
        let query = Query::Instances {
            product: Product::Cvm,
            region: region.into(),
            window: Window {
                offset: 0,
                limit: 20,
            },
        };
        assert_eq!(query.call().err().unwrap().class, Class::Input);
    }
    for limit in [0, 101] {
        let query = Query::Images {
            region: "ap-hongkong".into(),
            window: Window { offset: 0, limit },
        };
        assert_eq!(query.call().err().unwrap().class, Class::Input);
    }
    let query = Query::Shapes {
        region: "ap-hongkong".into(),
        zone: Some("ap-shanghai-1".into()),
    };
    assert_eq!(query.call().err().unwrap().class, Class::Input);
    let query = Query::Bundles {
        region: "ap-hongkong".into(),
        blueprint: Some("img-invalid".into()),
        window: Window {
            offset: 0,
            limit: 20,
        },
    };
    assert_eq!(query.call().err().unwrap().class, Class::Input);
}

#[test]
fn failures() {
    for (code, class) in [
        ("UnauthorizedOperation", Class::Permission),
        ("AuthFailure.TokenExpired", Class::Expired),
        ("RequestLimitExceeded", Class::Limit),
        ("SECRET_RESPONSE_FIXTURE", Class::Provider),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = receive(&stream);
            let body = refusal(code);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let credential = Credential {
            id: "AKID_FIXTURE".into(),
            key: "SECRET_FIXTURE".into(),
            token: None,
            expires: None,
        };
        let error = api::Client::new(&credential)
            .request(
                (&origin, "cvm.tencentcloudapi.com"),
                api::Operation::new("DescribeInstances", "2017-03-12", Some("ap-hongkong")),
                &serde_json::json!({"Limit":20}),
            )
            .err()
            .unwrap();
        worker.join().unwrap();
        let failure = super::super::Failure::capture(error, Class::Response);
        assert_eq!(failure.class, class);
        assert_eq!(failure.request.as_deref(), Some("fixture"));
        assert!(!format!("{failure:?} {failure}").contains("SECRET_RESPONSE_FIXTURE"));
        if class == Class::Provider {
            assert_eq!(failure.code, None);
        }
    }
}

fn refusal(code: &str) -> String {
    serde_json::json!({"Response":{"Error":{"Code":code,"Message":"SECRET_RESPONSE_FIXTURE"},"RequestId":"fixture"}}).to_string()
}
