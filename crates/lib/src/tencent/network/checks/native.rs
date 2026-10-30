use super::{Operation, Request};
use crate::tencent::{command::Deed, parse};

#[test]
fn native() {
    let argv: Vec<_> = [
        "network",
        "--region",
        "ap-hongkong",
        "group",
        "reset",
        "--id",
        "sg-fixture",
        "--version",
        "1",
        "--ingress",
        "tcp;443;0.0.0.0/0;accept",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let input = parse(&argv).unwrap();
    let Deed::Network(request) = input.deed else {
        panic!("network");
    };
    assert!(matches!(request.operation, Operation::Group(_)));
    assert!(request.call().unwrap().mutation());
    let mut bad = argv;
    bad.extend(["--raw-body".into(), "{}".into()]);
    assert!(parse(&bad).is_err());
    let no = Request {
        region: "ap-hongkong".into(),
        operation: Operation::Vpc(super::Vpc::Get(super::Id {
            id: "vpc-fixture".into(),
        })),
    };
    assert!(!no.call().unwrap().mutation());
}
