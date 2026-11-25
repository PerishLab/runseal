use super::{Deed, Input};
use clap::Parser;
use std::path::Path;

#[test]
fn files() {
    let input = Input::try_parse_from([
        "@github",
        "comment",
        "I_fixture",
        "--body-file",
        "/fixture/body",
        "--token-file",
        "/fixture/token",
        "--json",
    ])
    .unwrap();
    assert_eq!(input.file.as_deref(), Some(Path::new("/fixture/token")));
    let Deed::Comment { body, .. } = input.deed else {
        panic!("expected comment")
    };
    assert_eq!(body, Path::new("/fixture/body"));
}

#[test]
fn change() {
    let input = Input::try_parse_from([
        "@github",
        "project-change",
        "--change-file",
        "/fixture/change",
        "--token-file",
        "/fixture/token",
        "--json",
    ])
    .unwrap();
    assert!(matches!(input.deed, Deed::Change(_)));
    assert_eq!(input.file.as_deref(), Some(Path::new("/fixture/token")));
}

#[test]
fn primitives() {
    let amend = Input::try_parse_from([
        "@github",
        "--json",
        "issues",
        "amend",
        "I_fixture",
        "--body-file",
        "/fixture/body",
    ])
    .unwrap();
    assert!(amend.json);
    assert!(matches!(amend.deed, Deed::Issues(_)));
    let logs = Input::try_parse_from([
        "@github",
        "--json",
        "logs",
        "PerishLab",
        "wharf",
        "42",
        "--limit",
        "4096",
        "--output",
        "/fixture/job.log",
    ])
    .unwrap();
    assert!(logs.json);
    assert!(matches!(logs.deed, Deed::Logs(_)));
}

#[test]
fn stdin() {
    use crate::github::{Cause, Client, Outcome};
    let parsed = Input::try_parse_from([
        "@github",
        "--token-stdin",
        "--json",
        "compare",
        "PerishLab",
        "runseal",
        "1234567890123456789012345678901234567890",
        "abcdefabcdefabcdefabcdefabcdefabcdefabcd",
    ])
    .unwrap();
    assert!(parsed.stdin);
    assert!(matches!(parsed.deed, Deed::Compare(_)));
    assert!(
        Input::try_parse_from([
            "@github",
            "--token-stdin",
            "--token-file",
            "/fixture/token",
            "repository",
            "PerishLab",
            "runseal",
        ])
        .is_err()
    );
    for ok in [
        "github_pat_fixture",
        "github_pat_fixture\n",
        "github_pat_fixture\r\n",
    ] {
        assert!(Client::reader(ok.as_bytes()).is_ok());
    }
    let oversized = "a".repeat(4097);
    for bad in [
        "",
        "\n",
        "first\nsecond\n",
        "first\rsecond",
        oversized.as_str(),
    ] {
        let error = Client::reader(bad.as_bytes()).err().expect("rejected");
        assert_eq!(
            (error.outcome, error.cause),
            (Outcome::Unsent, Cause::Custody)
        );
    }
    let triggered = Input::try_parse_from([
        "@github",
        "--token-stdin",
        "--timeout",
        "5",
        "--json",
        "trigger",
        "PerishLab",
        "wharf",
        "--event",
        "follow-push",
        "--payload-file",
        "/fixture/payload.json",
    ])
    .unwrap();
    assert_eq!(triggered.timeout, Some(5));
    assert!(matches!(triggered.deed, Deed::Trigger(_)));
    for args in [
        &["@github", "repository", "PerishLab", "runseal"][..],
        &["@github", "repository", "show", "PerishLab", "runseal"][..],
        &[
            "@github",
            "--json",
            "repository",
            "create",
            "PerishLab",
            "simplex",
            "--visibility",
            "private",
            "--description",
            "Simplex programming language",
        ][..],
    ] {
        let parsed = Input::try_parse_from(args).unwrap();
        assert!(matches!(parsed.deed, Deed::Repository(_)));
    }
    for invalid in [
        &["@github", "repository"][..],
        &["@github", "repository", "PerishLab"][..],
        &["@github", "repository", "create", "PerishLab", "simplex"][..],
    ] {
        assert!(Input::try_parse_from(invalid).is_err());
    }
}
