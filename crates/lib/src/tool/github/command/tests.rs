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
