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
