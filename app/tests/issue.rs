mod support;

use support::{Seat, serve, text};

#[test]
fn created() {
    let (url, handle) = serve(
        "201 Created",
        r#"{"number":2,"state":"open","title":"new"}"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/keel",
        "issue",
        "create",
        "--title",
        "new",
    ]);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "2\topen\tnew");
    assert!(
        seen[0].starts_with("POST /api/v1/repos/PerishLab/keel/issues "),
        "{}",
        seen[0]
    );
}

#[test]
fn edited() {
    let (url, handle) = serve(
        "200 OK",
        r#"{"number":154,"state":"closed","title":"K1"}"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "issue",
        "edit",
        "PerishLab/keel#154",
        "--state",
        "closed",
    ]);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "154\tclosed\tK1");
    assert!(
        seen[0].starts_with("PATCH /api/v1/repos/PerishLab/keel/issues/154 "),
        "{}",
        seen[0]
    );
}

#[test]
fn notes() {
    let (url, handle) = serve(
        "200 OK",
        r#"[{"id":9,"body":"ack","user":{"login":"perish"}}]"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "issue",
        "comment",
        "list",
        "PerishLab/keel#154",
    ]);
    handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "9\tperish\tack");
}

#[test]
fn posted() {
    let (url, handle) = serve(
        "201 Created",
        r#"{"id":10,"body":"ack","user":{"login":"perish"}}"#,
        1,
    );
    let seat = Seat::new();
    seat.write(&url);
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "issue",
        "comment",
        "create",
        "PerishLab/keel#154",
        "--body",
        "ack",
    ]);
    let seen = handle.join().expect("server");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "10\tperish\tack");
    assert!(
        seen[0].starts_with("POST /api/v1/repos/PerishLab/keel/issues/154/comments "),
        "{}",
        seen[0]
    );
}

#[test]
fn untitled() {
    let seat = Seat::new();
    seat.write("http://127.0.0.1:1");
    let output = seat.run(&[
        ":perish",
        "@forgejo",
        "--repo",
        "PerishLab/keel",
        "issue",
        "create",
    ]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("--title"));
}
