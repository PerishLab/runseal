use super::*;
use crate::tencent::{Class, Identity, Outcome};

pub(super) fn identity() -> Identity {
    Identity {
        account: "123".into(),
        principal: "456".into(),
        kind: "CAMUser".into(),
        stamp: "fixture-stamp".into(),
        observed: 1,
        request: "identity-request".into(),
    }
}

fn page(page: u32) -> Request {
    Request::Entities { policy: 42, page }
}

fn receipt(request: &Request, value: Value) -> Receipt {
    record::receipt(request, identity(), &value).unwrap()
}

#[test]
fn inputs() {
    assert_eq!(
        page(2).params().unwrap(),
        json!({"PolicyId":42,"Page":2,"Rp":100,"EntityFilter":"All"})
    );
    assert_eq!(
        Request::Delete { policy: 42 }.params().unwrap(),
        json!({"PolicyId":[42]})
    );
    for request in [
        Request::Delete { policy: 0 },
        Request::Delete { policy: u64::MAX },
        Request::Entities {
            policy: 42,
            page: 0,
        },
        Request::Entities { policy: 0, page: 1 },
    ] {
        assert_eq!(request.validate().unwrap_err().class, Class::Input);
    }
}

#[test]
fn pages() {
    let rows: Vec<_> = (1..=100)
        .map(|n| json!({"Id":n.to_string(),"RelatedType":1,"Uin":n}))
        .collect();
    let result = receipt(
        &page(1),
        json!({"RequestId":"fixture-id","TotalNum":101,"List":rows}),
    );
    let Data::Entities(first) = result.data else {
        panic!("page")
    };
    assert_eq!(first.policy, 42);
    assert_eq!(first.next, Some(2));
    assert_eq!(first.items.len(), 100);
    assert!(first.complete);
    let row = json!({"Id":"1","RelatedType":3,"Uin":0,"Name":"PRIVATE_MARKER","AttachmentTime":"2021-11-11 19:38:01"});
    let result = receipt(
        &page(2),
        json!({"RequestId":"fixture-id","TotalNum":101,"List":[row]}),
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let Data::Entities(last) = result.data else {
        panic!("page")
    };
    assert_eq!(last.next, None);
    assert!(last.complete);
    assert_eq!(last.items[0].relation, Some(Relation::Role));
    assert_eq!(last.items[0].uin, Some(0));
    let result = receipt(
        &page(1),
        json!({"RequestId":"fixture-id","TotalNum":0,"List":[]}),
    );
    assert_eq!(result.outcome, Outcome::Observed);
    let Data::Entities(empty) = result.data else {
        panic!("page")
    };
    assert!(empty.complete && empty.items.is_empty());
    assert_eq!(empty.next, None);
}

#[test]
fn coverage() {
    for row in [
        json!({"Id":"1","RelatedType":99,"Uin":0}),
        json!({"Id":"1","RelatedType":null,"Uin":0}),
        json!({"Id":"1","RelatedType":2}),
        json!({"Id":"1","RelatedType":2,"Uin":null}),
        json!({"Id":"1","RelatedType":2,"Uin":0,"Extra":"PRIVATE_MARKER"}),
        json!({"Id":"1","RelatedType":2,"Uin":0,"Name":{"Secret":"PRIVATE_MARKER"}}),
    ] {
        let result = receipt(
            &page(1),
            json!({"RequestId":"fixture-id","TotalNum":1,"List":[row]}),
        );
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("PRIVATE_MARKER")
        );
        let Data::Entities(page) = result.data else {
            panic!("page")
        };
        assert!(!page.complete);
        assert_eq!(page.items.len(), 1);
    }
    let result = receipt(
        &page(1),
        json!({"RequestId":"fixture-id","TotalNum":0,"List":[],"Extra":"PRIVATE_MARKER"}),
    );
    let Data::Entities(page) = result.data else {
        panic!("page")
    };
    assert!(!page.complete);
}

#[test]
fn malformed() {
    let row = json!({"Id":"1","RelatedType":1,"Uin":1});
    for value in [
        json!({"RequestId":"fixture-id","List":[]}),
        json!({"RequestId":"fixture-id","TotalNum":null,"List":[]}),
        json!({"RequestId":"fixture-id","TotalNum":0}),
        json!({"RequestId":"fixture-id","TotalNum":0,"List":null}),
        json!({"RequestId":"fixture-id","TotalNum":1,"List":[]}),
        json!({"RequestId":"fixture-id","TotalNum":2,"List":[row.clone(),row]}),
        json!({"RequestId":"fixture-id","TotalNum":0,"List":[],"PolicyId":43}),
        json!({"RequestId":"fixture-id","TotalNum":1,"List":[{"Id":"PRIVATE_MARKER","RelatedType":1,"Uin":1}]}),
        json!({"RequestId":"fixture-id","TotalNum":1,"List":[{"Id":"1","RelatedType":"PRIVATE_MARKER","Uin":1}]}),
        json!({"RequestId":"fixture-id","TotalNum":1,"List":[{"Id":"1","RelatedType":1,"Uin":-1}]}),
    ] {
        let failure = record::receipt(&page(1), identity(), &value).unwrap_err();
        assert_eq!(failure.class, Class::Response);
        assert_eq!(failure.request.as_deref(), Some("fixture-id"));
        assert!(!format!("{failure:?} {failure}").contains("PRIVATE_MARKER"));
    }
}

#[test]
fn deletion() {
    for (body, complete) in [
        (json!({"RequestId":"fixture-id"}), true),
        (
            json!({"RequestId":"fixture-id","Extra":"PRIVATE_MARKER"}),
            false,
        ),
    ] {
        let result = receipt(&Request::Delete { policy: 42 }, body);
        assert_eq!(result.outcome, Outcome::Accepted);
        assert_eq!(result.region, "");
        let Data::Deleted {
            policy,
            complete: found,
        } = result.data
        else {
            panic!("delete")
        };
        assert_eq!(policy, 42);
        assert_eq!(found, complete);
    }
    for body in [json!({}), json!({"RequestId":"PRIVATE_MARKER\n"})] {
        let failure =
            record::receipt(&Request::Delete { policy: 42 }, identity(), &body).unwrap_err();
        assert!(failure.request.is_none());
        assert!(!format!("{failure:?}").contains("PRIVATE_MARKER"));
    }
}

#[test]
fn native() {
    for (verb, request) in [
        ("entities", page(1)),
        ("delete", Request::Delete { policy: 42 }),
    ] {
        let words = ["cam", "policy", verb, "--id", "42"].map(str::to_owned);
        let input = crate::tencent::parse(&words).unwrap();
        let crate::tencent::command::Deed::Cam(Cam::Policy(policy)) = input.deed else {
            panic!("CAM")
        };
        assert_eq!(convert(&policy), Some(request));
        assert!(super::request(&Cam::Policy(policy)).is_err());
        for extra in ["--raw-body", "--entity-filter", "--ids"] {
            let mut words = words.to_vec();
            words.extend([extra.into(), "PRIVATE_MARKER".into()]);
            assert!(crate::tencent::parse(&words).is_err());
        }
    }
}
