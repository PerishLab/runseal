use super::{envelope, profile};
use crate::support::{Seat, serve, text};
use serde_json::{Value, json};

#[test]
fn relative() {
    let secret = "RELATIVE_TOKEN_FIXTURE";
    let (url, handle) = serve("200 OK", &envelope(json!(secret), Value::Null), 1);
    let seat = Seat::new();
    profile(&seat, &url);
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--json",
        "--value-file",
        "../held/relative.token",
        "token",
        "account",
        "roll",
        "token-id",
    ]);
    handle.join().unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        std::fs::read_to_string(seat.home.join("relative.token")).unwrap(),
        format!("{secret}\n")
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["token"]["value_file"], "../held/relative.token");
    assert!(!text(&output.stdout).contains(secret));
    assert!(!text(&output.stderr).contains(secret));
}

#[cfg(unix)]
#[test]
fn ancestors() {
    use std::os::unix::fs::PermissionsExt;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let seat = Seat::new();
    profile(&seat, &format!("http://{}", listener.local_addr().unwrap()));
    std::fs::set_permissions(&seat.home, std::fs::Permissions::from_mode(0o777)).unwrap();
    let path = seat.home.join("refused.token");
    let output = seat.run(&[
        ":perish",
        "@cloudflare",
        "--value-file",
        path.to_str().unwrap(),
        "token",
        "account",
        "roll",
        "token-id",
    ]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unable to reserve token value file"));
    assert!(!path.exists());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
