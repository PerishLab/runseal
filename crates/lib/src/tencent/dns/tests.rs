use super::{Data, Fields, Kind, Operation, Reference, Request, Zone};
use serde_json::{Value, json};

pub(super) fn request(operation: Operation) -> Request {
    Request {
        zone: Zone::Id(42),
        operation,
    }
}

pub(super) fn fields() -> Fields {
    Fields {
        name: "api".into(),
        kind: Kind::A,
        value: "192.0.2.1".into(),
        line: "默认".into(),
        ttl: 600,
    }
}

pub(super) fn entry() -> Value {
    json!({"Id":9,"DomainId":42,"SubDomain":"api","RecordType":"A","Value":"192.0.2.1","RecordLine":"默认","RecordLineId":"0","TTL":600,"Enabled":1,"Weight":null,"MX":0,"SecretKey":"PRIVATE_MARKER"})
}

#[test]
fn qualification() {
    let named = Request {
        zone: Zone::Name("example.org".into()),
        operation: Operation::Domain,
    };
    let call = named.call().unwrap();
    assert_eq!(call.body, json!({"Domain":"example.org"}));
    assert_eq!(call.request().region, None);
    assert_eq!(call.request().service, "dnspod");
    let domain =
        json!({"DomainInfo":{"DomainId":42,"Domain":"example.org","Status":"ENABLE","TTL":600}});
    assert!(named.decode(&domain).is_ok());
    assert!(request(Operation::Domain).decode(&domain).is_ok());
    let mut wrong = domain;
    wrong["DomainInfo"]["DomainId"] = json!(99);
    assert!(request(Operation::Domain).decode(&wrong).is_err());
    assert!(
        Reference {
            domain: None,
            record: 9
        }
        .complete("123")
        .is_err()
    );
    let remote = Reference {
        domain: Some(42),
        record: 9,
    }
    .complete("123")
    .unwrap();
    assert_eq!(remote.domain, 42);
    assert_eq!(remote.record, 9);
    assert_eq!(remote.account, "123");
}

#[test]
fn exact() {
    let request = request(Operation::Get { id: 9 });
    let call = request.call().unwrap();
    assert_eq!(call.body, json!({"DomainId":42,"RecordId":9}));
    assert_eq!(call.action, "DescribeRecord");
    let value = json!({"RecordInfo":entry()});
    let Data::Record(found) = request.decode(&value).unwrap() else {
        panic!("record");
    };
    assert_eq!(found.reference.complete("123").unwrap().domain, 42);
    assert!(
        !serde_json::to_string(&found)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let mut wrong = value.clone();
    wrong["RecordInfo"]["Id"] = json!(10);
    assert!(request.decode(&wrong).is_err());
    let mut wrong = value.clone();
    wrong["RecordInfo"]["DomainId"] = json!(99);
    assert!(request.decode(&wrong).is_err());
    let mut missing = value;
    missing["RecordInfo"]
        .as_object_mut()
        .unwrap()
        .remove("DomainId");
    assert!(request.decode(&missing).is_err());
}

#[test]
fn pages() {
    let request = request(Operation::List {
        offset: 0,
        limit: 1,
    });
    let call = request.call().unwrap();
    assert_eq!(call.body["ErrorOnEmpty"], "no");
    let item = json!({"RecordId":9,"Name":"@","Type":"NS","Value":"ns.example.org.","Line":"默认","LineId":"0","TTL":600,"Status":"ENABLE","DefaultNS":true});
    let value = json!({"RecordList":[item],"RecordCountInfo":{"TotalCount":2,"ListCount":1}});
    let Data::Records(page) = request.decode(&value).unwrap() else {
        panic!("page");
    };
    assert_eq!(page.next, Some(1));
    assert_eq!(page.items[0].kind, "NS");
    assert_eq!(page.items[0].default, Some(true));
    let empty = json!({"RecordList":[],"RecordCountInfo":{"TotalCount":0,"ListCount":0}});
    assert!(request.decode(&empty).is_ok());
    let mut delayed = empty.clone();
    delayed["RecordCountInfo"]["TotalCount"] = json!(1);
    assert!(request.decode(&delayed).is_err());
    let mut missing = empty;
    missing.as_object_mut().unwrap().remove("RecordCountInfo");
    assert!(request.decode(&missing).is_err());
    let named = Request {
        zone: Zone::Name("example.org".into()),
        operation: Operation::List {
            offset: 0,
            limit: 1,
        },
    };
    let Data::Records(page) = named.decode(&value).unwrap() else {
        panic!("page");
    };
    assert!(page.items[0].reference.complete("123").is_err());
    let mut duplicate = value;
    let first = duplicate["RecordList"][0].clone();
    duplicate["RecordList"].as_array_mut().unwrap().push(first);
    assert!(request.decode(&duplicate).is_err());
}

#[test]
fn acknowledgements() {
    let create = request(Operation::Create(fields()));
    let call = create.call().unwrap();
    assert!(call.body.get("ClientToken").is_none());
    assert!(call.body.get("Version").is_none());
    assert_eq!(call.body["RecordType"], "A");
    let Data::Acknowledged(reference) = create.decode(&json!({"RecordId":9})).unwrap() else {
        panic!("ack");
    };
    assert_eq!(reference.complete("123").unwrap().record, 9);
    let named = Request {
        zone: Zone::Name("example.org".into()),
        operation: Operation::Create(fields()),
    };
    let Data::Acknowledged(reference) = named.decode(&json!({"RecordId":9})).unwrap() else {
        panic!("ack");
    };
    assert!(reference.complete("123").is_err());
    assert!(
        request(Operation::Modify {
            id: 9,
            fields: fields()
        })
        .decode(&json!({"RecordId":10}))
        .is_err()
    );
    assert!(create.decode(&json!({"RequestId":"fixture"})).is_err());
    assert!(matches!(
        request(Operation::Delete { id: 9 })
            .decode(&json!({}))
            .unwrap(),
        Data::Acknowledged(_)
    ));
}

#[test]
fn bounds() {
    for domain in [
        "Example.org",
        "example..org",
        "example.org/secret",
        "-example.org",
        "example.org.",
    ] {
        assert!(
            Request {
                zone: Zone::Name(domain.into()),
                operation: Operation::Domain
            }
            .call()
            .is_err()
        );
    }
    assert!(
        Request {
            zone: Zone::Id(0),
            operation: Operation::Domain
        }
        .call()
        .is_err()
    );
    for limit in [0, 3001] {
        assert!(
            request(Operation::List { offset: 0, limit })
                .call()
                .is_err()
        );
    }
    assert!(
        request(Operation::List {
            offset: 0,
            limit: 3000
        })
        .call()
        .is_ok()
    );
    assert!(request(Operation::Get { id: 0 }).call().is_err());
    let mut invalid = fields();
    invalid.kind = Kind::Aaaa;
    assert!(request(Operation::Create(invalid)).call().is_err());
}
