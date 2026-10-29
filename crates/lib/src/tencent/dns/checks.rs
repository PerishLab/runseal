use super::{Data, Operation, tests::request};
use crate::tencent::{command::Deed, parse};
use serde_json::json;

#[test]
fn cli() {
    let words = ["dns", "record", "get", "--domain-id", "42", "--id", "9"].map(str::to_owned);
    let parsed = parse(&words).unwrap();
    let Deed::Dns(deed) = parsed.deed else {
        panic!("dns");
    };
    let converted = crate::tencent::command::dns::request(&deed).unwrap();
    assert_eq!(
        converted.call().unwrap().body,
        request(Operation::Get { id: 9 }).call().unwrap().body
    );
    let conflict = [
        "dns",
        "domain",
        "--domain",
        "example.org",
        "--domain-id",
        "42",
    ]
    .map(str::to_owned);
    assert!(parse(&conflict).is_err());
    let region = [
        "dns",
        "domain",
        "--domain",
        "example.org",
        "--region",
        "ap-hongkong",
    ]
    .map(str::to_owned);
    assert!(parse(&region).is_err());
    let raw = [
        "dns",
        "record",
        "get",
        "--domain-id",
        "42",
        "--id",
        "9",
        "--raw-body",
        "{}",
    ]
    .map(str::to_owned);
    assert!(parse(&raw).is_err());
    let list = request(Operation::List {
        offset: 0,
        limit: 20,
    });
    let page = json!({"RecordList":[],"RecordCountInfo":{"TotalCount":0,"ListCount":0}});
    assert!(matches!(list.decode(&page).unwrap(), Data::Records(_)));
    assert!(request(Operation::Get { id: 9 }).decode(&page).is_err());
}
