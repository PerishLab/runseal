use super::{Cause, Client, Outcome, Wire};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TOKEN: &str = "github_pat_fixture";
const PROJECT: &str = r#"{"data":{"organization":{"login":"PerishLab","projectV2":{"id":"PVT_example","number":6,"title":"Observation"}}}}"#;
const PARTIAL: &str = r#"{"data":{"organization":{"login":"PerishLab","projectV2":{"id":"PVT_example","number":6,"title":"Observation"}}},"errors":[{"message":"github_pat_fixture"}]}"#;

struct Server {
    client: Client,
    thread: JoinHandle<Vec<String>>,
}

fn serve(replies: Vec<Option<String>>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut client = Client::token(TOKEN).unwrap();
    client.wire = Wire::new(format!("http://{}", listener.local_addr().unwrap()));
    let thread = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let Some(mut stream) = accept(&listener) else {
                break;
            };
            requests.push(read(&stream));
            if let Some(reply) = reply {
                stream.write_all(reply.as_bytes()).unwrap();
            }
        }
        requests
    });
    Server { client, thread }
}

fn accept(listener: &TcpListener) -> Option<TcpStream> {
    let until = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < until {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                return Some(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(4));
            }
            Err(error) => panic!("{error}"),
        }
    }
    None
}

fn read(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line);
    }
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    headers.push_str(&String::from_utf8(body).unwrap());
    headers
}

fn reply(status: &str, headers: &str, body: &str) -> Option<String> {
    Some(format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    ))
}

#[test]
fn project() {
    let target = serve(vec![
        None,
        reply("200 OK", "X-GitHub-Request-Id: 1234:ABCD:\r\n", PROJECT),
    ]);
    let result = target.client.project("PerishLab", 6).unwrap();
    assert_eq!(result.outcome, Outcome::Observed);
    assert_eq!(result.value.id, "PVT_example");
    assert_eq!(result.evidence.attempts, 2);
    assert_eq!(result.evidence.request.as_deref(), Some("1234:ABCD:"));
    for request in target.thread.join().unwrap() {
        assert!(request.starts_with("POST /graphql HTTP/1.1"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer github_pat_fixture")
        );
        assert!(request.contains("query RunsealProject"));
    }
}

#[test]
fn write() {
    let target = serve(vec![None, reply("200 OK", "", PROJECT)]);
    let error = target.client.close("I_example").unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert_eq!(error.cause, Cause::Network);
    assert_eq!(error.evidence.attempts, 1);
    let requests = target.thread.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("mutation RunsealClose"));
}

#[test]
fn rate() {
    let target = serve(vec![reply(
        "403 Forbidden",
        "Retry-After: 12\r\nX-RateLimit-Remaining: 0\r\n",
        "opaque",
    )]);
    let error = target.client.project("PerishLab", 6).unwrap_err();
    assert_eq!(error.outcome, Outcome::Refused);
    assert_eq!(error.cause, Cause::Limited);
    assert_eq!(error.evidence.wait, Some(12));
    assert_eq!(error.evidence.attempts, 1);
    assert_eq!(target.thread.join().unwrap().len(), 1);
}

#[test]
fn partial() {
    let target = serve(vec![reply(
        "200 OK",
        "X-GitHub-Request-Id: github_pat_fixture\r\n",
        PARTIAL,
    )]);
    let error = target.client.project("PerishLab", 6).unwrap_err();
    assert_eq!(error.cause, Cause::Graphql);
    assert!(error.evidence.partial);
    assert!(error.evidence.request.is_none());
    assert!(!serde_json::to_string(&error).unwrap().contains(TOKEN));
    assert_eq!(target.thread.join().unwrap().len(), 1);
}

#[test]
fn repository() {
    let result =
        r#"{"id":14,"full_name":"PerishLab/runseal","private":true,"default_branch":"main"}"#;
    let target = serve(vec![reply("200 OK", "", result)]);
    let local = target
        .client
        .repository("PerishLab", "../runseal")
        .unwrap_err();
    assert_eq!(local.outcome, Outcome::Unsent);
    assert_eq!(
        target
            .client
            .repository("perishlab", "Runseal")
            .unwrap()
            .value
            .id,
        14
    );
    let requests = target.thread.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /repos/perishlab/Runseal HTTP/1.1"));
}

#[test]
fn confirmed() {
    let body = r#"{"data":{"closeIssue":{"issue":{"id":"I_example","state":"CLOSED","reason":"COMPLETED","repository":{"id":"R_example"}}}}}"#;
    let target = serve(vec![reply("200 OK", "", body), reply("200 OK", "", body)]);
    let closed = target.client.close("I_example").unwrap();
    assert_eq!(closed.value.state, "CLOSED");
    assert_eq!(closed.outcome, Outcome::Observed);
    let wrong = target.client.close("I_other").unwrap_err();
    assert_eq!(wrong.outcome, Outcome::Unknown);
    assert_eq!(wrong.cause, Cause::Shape);
    assert_eq!(target.thread.join().unwrap().len(), 2);
}

#[test]
fn truncated() {
    let short = "HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 90\r\nRetry-After: 25\r\n\r\nshort";
    let target = serve(vec![Some(short.into()), reply("200 OK", "", PROJECT)]);
    let error = target.client.project("PerishLab", 6).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert_eq!(error.cause, Cause::Read);
    assert_eq!(error.evidence.status, Some(503));
    assert_eq!(target.thread.join().unwrap().len(), 1);
}

mod issues;

mod pulls;

mod actions;

mod delivery;

mod project;

#[test]
fn token() {
    for value in [TOKEN, "ghs_1234_eyJhIjoiYiJ9.eyJjIjoiZCJ9.signature"] {
        assert!(Client::token(value).is_ok());
    }
    for value in ["", "abc def", "abc\r\nAuthorization: other", "abc\0def"] {
        let Some(error) = Client::token(value).err() else {
            panic!("accepted invalid bearer");
        };
        assert_eq!(error.outcome, Outcome::Unsent);
        assert_eq!(error.cause, Cause::Custody);
    }
}

mod source;

mod structure;

mod schema;

mod catalogue;

#[test]
fn snapshot() {
    use crate::github::{Target, snapshot::Cursors};
    let empty = r#"{"totalCount":0,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}"#;
    let valid = r#"{"data":{"repository":{"id":"R_fixture","nameWithOwner":"PerishLab/runseal",
        "issue":{"id":"I_fixture","number":9,"url":"https://github.com/PerishLab/runseal/issues/9",
            "title":"Issue","body":"Proof","state":"OPEN","updatedAt":"2026-11-24T08:00:00Z",
            "issueType":{"name":"Task"},"repository":{"nameWithOwner":"PerishLab/runseal"},
            "parent":null,"subIssues":EMPTY,"subIssuesSummary":{"total":0,"completed":0,"percentCompleted":0},
            "blockedBy":EMPTY,"blocking":EMPTY,"timelineItems":EMPTY,"comments":EMPTY}
    }}}"#.replace("EMPTY", empty);
    let mut stalled: serde_json::Value = serde_json::from_str(&valid).unwrap();
    stalled["data"]["repository"]["issue"]["blocking"]["pageInfo"]["hasNextPage"] = true.into();
    stalled["data"]["repository"]["issue"]["blocking"]["pageInfo"]["endCursor"] =
        "no-progress".into();
    let server = serve(vec![
        reply("200 OK", "", &valid),
        reply("200 OK", "", &stalled.to_string()),
        reply("200 OK", "", &valid.replace("/issues/9", "/issues/10")),
    ]);
    let target = Target::new("PerishLab", "runseal", 9).unwrap();
    let cursors = Cursors::default();
    let receipt = server.client.snapshot(&target, 5, &cursors).unwrap();
    assert_eq!(receipt.value.kind.unwrap().name, "Task");
    assert_eq!(receipt.value.blocking.total, 0);
    assert!(receipt.value.references.nodes.is_empty());
    for _ in 0..2 {
        assert_eq!(
            server
                .client
                .snapshot(&target, 5, &cursors)
                .unwrap_err()
                .cause,
            Cause::Shape
        );
    }
    let sent = server.thread.join().unwrap();
    assert!(sent[0].contains("CROSS_REFERENCED_EVENT"));
}
