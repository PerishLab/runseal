use super::{survey::Probe, surveys::call, wire};
use crate::tencent::{Class, Failure, Outcome, api};
use serde_json::{Value, json};
use std::{
    io::{ErrorKind, Write},
    net::TcpListener,
    thread,
};

fn invoke(probe: Probe, payload: Value) -> Result<super::Data, super::Error> {
    let call = call(probe);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let received = wire::receive(&stream);
        let body = json!({"Response":payload}).to_string();
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
    let body = api::Client::new(&credential).request(
        (&origin, "vpc.tencentcloudapi.com"),
        api::Operation::new(call.action, "2017-03-12", Some("ap-hongkong")),
        &call.body,
    );
    let (listener, received) = worker.join().unwrap();
    listener.set_nonblocking(true).unwrap();
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    let lower = received.to_ascii_lowercase();
    assert!(lower.contains("host: vpc.tencentcloudapi.com"));
    assert!(lower.contains("x-tc-version: 2017-03-12"));
    assert!(lower.contains("x-tc-region: ap-hongkong"));
    assert!(lower.contains("authorization: tc3-hmac-sha256"));
    assert!(lower.contains("x-tc-token: token_fixture"));
    assert!(lower.contains(&format!(
        "x-tc-action: {}",
        call.action.to_ascii_lowercase()
    )));
    assert!(received.ends_with(&call.body.to_string()));
    match body {
        Ok(body) => call.decode(&body.0).map_err(|fault| super::Error {
            fault,
            outcome: Outcome::Unknown,
        }),
        Err(error) => Err(super::Error::remote(Failure::capture(
            error,
            Class::Response,
        ))),
    }
}

#[test]
fn signed() {
    for (probe, payload) in [
        (
            Probe::Vpc,
            json!({"RequestId":"vpc-receipt", "ResourceDashboardSet":[{"VpcId":"vpc-fixture","Ip":0}]}),
        ),
        (
            Probe::Subnet,
            json!({"RequestId":"subnet-receipt", "ResourceStatisticsSet":[{"VpcId":"vpc-fixture","SubnetId":"subnet-fixture","Ip":0,"ResourceStatisticsItemSet":[]}]}),
        ),
        (
            Probe::Group,
            json!({"RequestId":"group-receipt", "SecurityGroupAssociationStatisticsSet":[{"SecurityGroupId":"sg-fixture","TotalCount":0,"SG":0,"InstanceStatistics":[]}]}),
        ),
    ] {
        let data = invoke(probe, payload).unwrap();
        assert!(matches!(data, super::Data::Survey { entry: Some(_) }));
    }
}

#[test]
fn refusal() {
    let error = invoke(Probe::Vpc, json!({"RequestId":"absence-fixture","Error":{"Code":"ResourceNotFound", "Message":"SECRET_FIXTURE"}})).unwrap_err();
    assert_eq!(error.fault.code.as_deref(), Some("ResourceNotFound"));
    assert_eq!(error.fault.request.as_deref(), Some("absence-fixture"));
    assert_eq!(error.outcome, Outcome::Rejected);
    assert!(!error.to_string().contains("SECRET_FIXTURE"));
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("SECRET_FIXTURE")
    );
    let error = invoke(Probe::Group, json!({"RequestId":"partial-fixture"})).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
}
