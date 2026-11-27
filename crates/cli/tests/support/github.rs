#![cfg(unix)]

use rustls::{
    ServerConfig, ServerConnection, StreamOwned,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, pem::PemObject},
};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const ROOT: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIBrjCCAVWgAwIBAgIUbP9fyBbul6ID/TNO017t4aSCMHowCgYIKoZIzj0EAwIw
JTEjMCEGA1UEAwwaUnVuc2VhbCBDT05ORUNUIGZpeHR1cmUgQ0EwHhcNMjYxMTI3
MDcxOTM2WhcNNDYxMTIyMDcxOTM2WjAlMSMwIQYDVQQDDBpSdW5zZWFsIENPTk5F
Q1QgZml4dHVyZSBDQTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABG9jsXFj27he
V5+GbMlU+8Ie0uMQVAa1/qHjC9g/PuSPVted+hyJuk0FM2cmZ37cqsXjS+CLxEXr
+dpmD5gcJGujYzBhMB0GA1UdDgQWBBSiSWOdHQoV1OXq5oOVInVrHFkDpzAfBgNV
HSMEGDAWgBSiSWOdHQoV1OXq5oOVInVrHFkDpzAPBgNVHRMBAf8EBTADAQH/MA4G
A1UdDwEB/wQEAwIBBjAKBggqhkjOPQQDAgNHADBEAiBe/Mm0izYe5jsOhdwxv+co
G97Pzs7Wt6jLreuR4tUKlQIgDlG3OErQc6JyabfWse76lJA0jDNvsJQqrTijzm6B
XHM=
-----END CERTIFICATE-----
"#;
const LEAF: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIByDCCAW6gAwIBAgIBATAKBggqhkjOPQQDAjAlMSMwIQYDVQQDDBpSdW5zZWFs
IENPTk5FQ1QgZml4dHVyZSBDQTAeFw0yNDAxMDEwMDAwMDBaFw00NjAxMDEwMDAw
MDBaMCIxIDAeBgNVBAMMF1J1bnNlYWwgQ09OTkVDVCBmaXh0dXJlMFkwEwYHKoZI
zj0CAQYIKoZIzj0DAQcDQgAEHW92rw2wpsVw2Lqm6zFnOiD+gOV0JOIvlTO4CBX8
BTsAuiMPfrbMXIK9B06dmCFmQ5GoKqQ/yV/OvNGf5j01HKOBkTCBjjAMBgNVHRMB
Af8EAjAAMA4GA1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDATAZBgNV
HREEEjAQgg5hcGkuZ2l0aHViLmNvbTAdBgNVHQ4EFgQUtmlrE5J4uVH/73rQVIgn
ljWgV9QwHwYDVR0jBBgwFoAUokljnR0KFdTl6uaDlSJ1axxZA6cwCgYIKoZIzj0E
AwIDSAAwRQIgcDVzhvHQgIdWOmcFZghdCwfnuKWnl1A8w+blsL/H4OMCIQCWs+TN
gBGXQwHtAZUK8sNH0BjjNTxBxvmBTfKIoenbzQ==
-----END CERTIFICATE-----
"#;
const WRONG: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIBxzCCAW2gAwIBAgIBAjAKBggqhkjOPQQDAjAlMSMwIQYDVQQDDBpSdW5zZWFs
IENPTk5FQ1QgZml4dHVyZSBDQTAeFw0yNDAxMDEwMDAwMDBaFw00NjAxMDEwMDAw
MDBaMCIxIDAeBgNVBAMMF1J1bnNlYWwgQ09OTkVDVCBmaXh0dXJlMFkwEwYHKoZI
zj0CAQYIKoZIzj0DAQcDQgAEHW92rw2wpsVw2Lqm6zFnOiD+gOV0JOIvlTO4CBX8
BTsAuiMPfrbMXIK9B06dmCFmQ5GoKqQ/yV/OvNGf5j01HKOBkDCBjTAMBgNVHRMB
Af8EAjAAMA4GA1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDATAYBgNV
HREEETAPgg13cm9uZy5leGFtcGxlMB0GA1UdDgQWBBS2aWsTkni5Uf/vetBUiCeW
NaBX1DAfBgNVHSMEGDAWgBSiSWOdHQoV1OXq5oOVInVrHFkDpzAKBggqhkjOPQQD
AgNIADBFAiA2yWKGgibgrO2EA9irfgexI1xcoRPEoWULRwyv1N+C+QIhAPioTyT/
crdYikOZ16jrReELYOXiNc/I3Ak0vEZUJUJp
-----END CERTIFICATE-----
"#;
const KEY: &[u8] = &[
    48, 129, 135, 2, 1, 0, 48, 19, 6, 7, 42, 134, 72, 206, 61, 2, 1, 6, 8, 42, 134, 72, 206, 61, 3,
    1, 7, 4, 109, 48, 107, 2, 1, 1, 4, 32, 156, 83, 154, 8, 127, 24, 238, 180, 115, 150, 245, 111,
    136, 48, 85, 222, 80, 119, 97, 210, 103, 17, 148, 44, 10, 215, 105, 139, 1, 109, 218, 200, 161,
    68, 3, 66, 0, 4, 29, 111, 118, 175, 13, 176, 166, 197, 112, 216, 186, 166, 235, 49, 103, 58,
    32, 254, 128, 229, 116, 36, 226, 47, 149, 51, 184, 8, 21, 252, 5, 59, 0, 186, 35, 15, 126, 182,
    204, 92, 130, 189, 7, 78, 157, 152, 33, 102, 67, 145, 168, 42, 164, 63, 201, 95, 206, 188, 209,
    159, 230, 61, 53, 28,
];
const TOKEN: &[u8] = b"github_fixture_token\n";

#[test]
fn connect() {
    let temp = private();
    let root = temp.path().join("root.pem");
    runseal::custody::Private::current()
        .create(&root, ROOT)
        .unwrap();
    let bundle = temp.path().join("bundle.pem");
    let mut roots = ROOT.to_vec();
    roots.extend_from_slice(ROOT);
    runseal::custody::Private::current()
        .create(&bundle, &roots)
        .unwrap();

    let receipt = exchange(&root, Some(&bundle), LEAF, 1);
    assert_eq!(receipt["outcome"], "observed");
    assert_eq!(receipt["value"]["full_name"], "PerishLab/fixture");
    assert_eq!(receipt["evidence"]["attempts"], 1);

    for (explicit, leaf) in [(None, LEAF), (Some(root.as_path()), WRONG)] {
        let error = exchange(&root, explicit, leaf, 2);
        assert_eq!(error["outcome"], "unknown");
        assert_eq!(error["cause"], "network");
        assert_eq!(error["evidence"]["attempts"], 2);
    }
}

#[test]
fn invalid() {
    let temp = private();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let empty = temp.path().join("empty.pem");
    let mixed = temp.path().join("mixed.pem");
    let mut contents = ROOT.to_vec();
    contents.extend_from_slice(b"-----BEGIN CERTIFICATE-----\nYQ==\n-----END CERTIFICATE-----\n");
    runseal::custody::Private::current()
        .create(&empty, b"")
        .unwrap();
    runseal::custody::Private::current()
        .create(&mixed, &contents)
        .unwrap();

    for (path, cause) in [
        (empty, "input"),
        (mixed, "input"),
        (temp.path().join("missing.pem"), "custody"),
    ] {
        let error = request(&proxy, &path, Some(&path));
        assert_eq!(error["cause"], cause);
        assert_eq!(error["outcome"], "unsent");
        assert_eq!(error["evidence"]["attempts"], 0);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

fn private() -> tempfile::TempDir {
    let directory = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

fn exchange(ambient: &Path, explicit: Option<&Path>, leaf: &[u8], attempts: usize) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let certificate = CertificateDer::from_pem_slice(leaf).unwrap();
    let key = PrivatePkcs8KeyDer::from(KEY.to_vec());
    let config = Arc::new(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key.into())
            .unwrap(),
    );
    let success = explicit.is_some() && leaf == LEAF;
    let server = thread::spawn(move || {
        for _ in 0..attempts {
            respond(accept(&listener), config.clone(), success);
        }
    });
    let receipt = request(&proxy, ambient, explicit);
    server.join().unwrap();
    receipt
}

fn request(proxy: &str, ambient: &Path, explicit: Option<&Path>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_runseal"));
    command
        .args([
            ":",
            "@github",
            "--token-stdin",
            "--timeout",
            "2",
            "--json",
            "repository",
            "show",
            "PerishLab",
            "fixture",
        ])
        .env("ALL_PROXY", proxy)
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .env("SSL_CERT_FILE", ambient)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = explicit {
        command.arg("--root-ca-file").arg(path);
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(TOKEN).unwrap();
    let output = child.wait_with_output().unwrap();
    let receipt: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {output:?}"));
    let kind = if output.status.success() {
        "github"
    } else {
        "error"
    };
    receipt[kind].clone()
}

fn accept(listener: &TcpListener) -> TcpStream {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < until, "no CONNECT reached the fixture");
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("{error}"),
        }
    }
}

fn respond(mut stream: TcpStream, config: Arc<ServerConfig>, success: bool) {
    let head = headers(&mut stream);
    assert!(
        head.starts_with("CONNECT api.github.com:443 HTTP/1.1\r\n"),
        "{head}"
    );
    assert!(!head.to_ascii_lowercase().contains("authorization:"));
    stream
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .unwrap();

    let mut tls = ServerConnection::new(config).unwrap();
    let handshake = tls.complete_io(&mut stream);
    assert_eq!(tls.server_name(), Some("api.github.com"));
    if !success {
        assert!(handshake.is_err(), "unexpected TLS acceptance");
        return;
    }
    handshake.unwrap();
    let mut stream = StreamOwned::new(tls, stream);
    let head = headers(&mut stream);
    assert!(
        head.starts_with("GET /repos/PerishLab/fixture HTTP/1.1\r\n"),
        "{head}"
    );
    assert!(
        head.to_ascii_lowercase()
            .contains("authorization: bearer github_fixture_token\r\n")
    );
    let body =
        r#"{"id":42,"full_name":"PerishLab/fixture","private":false,"default_branch":"main"}"#;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}

fn headers(stream: impl std::io::Read) -> String {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line == "\r\n" {
            return head;
        }
        head.push_str(&line);
    }
}
