use super::super::firewall::{Change, Rule};
use super::*;

fn rules(offset: u32, limit: u32) -> Request {
    request(Operation::Firewall(
        super::super::firewall::Operation::List {
            instance: "lhins-fixture".into(),
            window: Window { offset, limit },
        },
    ))
}

fn change() -> Change {
    Change {
        instance: "lhins-fixture".into(),
        version: 7,
        rule: Rule {
            protocol: "TCP".into(),
            port: 443,
            cidr: "0.0.0.0/0".into(),
            action: "ACCEPT".into(),
            description: None,
        },
    }
}

fn response() -> Value {
    json!({"RequestId":"firewall-read", "TotalCount":1,"FirewallVersion":7,"FirewallRuleSet":[{
        "Protocol":"TCP","Port":"22,80-81","CidrBlock":"0.0.0.0/0","Ipv6CidrBlock":"","Action":"ACCEPT","AppType":"custom","FirewallRuleDescription":null
    }]})
}

#[test]
fn paging() {
    let query = rules(0, 1);
    let mut body = response();
    body["TotalCount"] = 2.into();
    let receipt = super::protocol::exchange(query, body.clone());
    assert_eq!(receipt.outcome, Outcome::Observed);
    assert_eq!(receipt.identity.account, "123");
    assert_eq!(receipt.region, "na-ashburn");
    assert_eq!(receipt.action, "DescribeFirewallRules");
    let Data::Rules(page) = receipt.data else {
        panic!()
    };
    assert_eq!(page.instance, "lhins-fixture");
    assert_eq!(page.total, 2);
    assert_eq!(page.version, 7);
    assert_eq!(page.next, Some(1));
    assert!(page.complete);
    assert!(page.entries[0].description.is_none());
    let Data::Rules(page) = rules(1, 1).receipt(identity(), &body).unwrap().data else {
        panic!()
    };
    assert_eq!(page.next, None);
    assert_eq!(page.offset, 1);
    body["FirewallRuleSet"] = json!([]);
    body["TotalCount"] = 0.into();
    body["FirewallVersion"] = 0.into();
    let Data::Rules(page) = rules(0, 100).receipt(identity(), &body).unwrap().data else {
        panic!()
    };
    assert!(page.entries.is_empty());
    assert_eq!(page.version, 0);
    assert!(page.complete);
}

#[test]
fn unknowns() {
    for field in ["FirewallRuleSet", "TotalCount", "FirewallVersion"] {
        let mut body = response();
        body.as_object_mut().unwrap().remove(field);
        assert!(rules(0, 20).receipt(identity(), &body).is_err());
        body[field] = Value::Null;
        assert!(rules(0, 20).receipt(identity(), &body).is_err());
    }
    let mut body = response();
    body["FirewallRuleSet"][0]["FutureField"] = "PRIVATE_MARKER".into();
    body["FirewallRuleSet"][0]["Protocol"] = "PRIVATE_PROTOCOL".into();
    body["FirewallRuleSet"][0]["CidrBlock"] = Value::Null;
    body["FutureField"] = "PRIVATE_MARKER".into();
    let receipt = rules(0, 20).receipt(identity(), &body).unwrap();
    assert!(
        !serde_json::to_string(&receipt)
            .unwrap()
            .contains("PRIVATE_")
    );
    let Data::Rules(page) = receipt.data else {
        panic!()
    };
    assert!(!page.complete);
    assert_eq!(page.schema.fields, vec!["FutureField"]);
    assert!(page.entries[0].cidr.is_none());
    assert_eq!(
        page.entries[0].protocol.as_ref().unwrap().value(),
        "UNKNOWN"
    );
}

#[test]
fn conflicts() {
    let query = rules(0, 20);
    let mut empty = response();
    empty["FirewallRuleSet"] = json!([]);
    assert!(query.receipt(identity(), &empty).is_err());
    let mut excess = response();
    excess["TotalCount"] = 0.into();
    assert!(query.receipt(identity(), &excess).is_err());
    let mut duplicate = response();
    let mut row = duplicate["FirewallRuleSet"][0].clone();
    row["FirewallRuleDescription"] = "different annotation".into();
    duplicate["FirewallRuleSet"]
        .as_array_mut()
        .unwrap()
        .push(row);
    duplicate["TotalCount"] = 2.into();
    assert!(query.receipt(identity(), &duplicate).is_err());
    for (field, value) in [("Port", "PRIVATE_PORT"), ("CidrBlock", "PRIVATE_CIDR")] {
        let mut body = response();
        body["FirewallRuleSet"][0][field] = value.into();
        assert!(query.receipt(identity(), &body).is_err());
    }
    for limit in [0, 101] {
        assert!(rules(0, limit).validate().is_err());
    }
}

#[test]
fn mutations() {
    for create in [false, true] {
        let mut target = change();
        if create {
            target.rule.description = Some("hardrig HTTPS".into());
        }
        let operation = if create {
            super::super::firewall::Operation::Create(target)
        } else {
            super::super::firewall::Operation::Delete(target)
        };
        let query = request(Operation::Firewall(operation));
        let call = query.call().unwrap();
        assert_eq!(call.body["InstanceId"], "lhins-fixture");
        assert_eq!(call.body["FirewallVersion"], 7);
        assert_eq!(call.body["FirewallRules"].as_array().unwrap().len(), 1);
        assert_eq!(call.body["FirewallRules"][0]["Port"], "443");
        assert_eq!(call.body["FirewallRules"][0]["Protocol"], "TCP");
        assert_eq!(call.body["FirewallRules"][0]["CidrBlock"], "0.0.0.0/0");
        assert_eq!(call.body["FirewallRules"][0]["Action"], "ACCEPT");
        assert_eq!(
            call.body["FirewallRules"][0]
                .get("FirewallRuleDescription")
                .is_some(),
            create
        );
        assert_eq!(
            call.action,
            if create {
                "CreateFirewallRules"
            } else {
                "DeleteFirewallRules"
            }
        );
        let receipt = super::protocol::exchange(query.clone(), json!({"RequestId":"mutation-id"}));
        assert_eq!(receipt.outcome, Outcome::Accepted);
        let Data::Changed {
            requested,
            complete,
        } = receipt.data
        else {
            panic!()
        };
        assert_eq!(requested.version, 7);
        assert!(complete);
        assert!(query.receipt(identity(), &json!({})).is_err());
        let Data::Changed { complete, .. } = query
            .receipt(
                identity(),
                &json!({"RequestId":"mutation-id","Future":true}),
            )
            .unwrap()
            .data
        else {
            panic!()
        };
        assert!(!complete);
    }
}

#[test]
fn input() {
    let mut cases = Vec::new();
    let mut target = change();
    target.version = 0;
    cases.push(target);
    let mut target = change();
    target.rule.port = 0;
    cases.push(target);
    let mut target = change();
    target.instance = "ins-foreign".into();
    cases.push(target);
    for protocol in ["ALL", "ICMP", "tcp", "PRIVATE"] {
        let mut target = change();
        target.rule.protocol = protocol.into();
        cases.push(target);
    }
    for cidr in ["::/0", "10.0.0.1/24", "0.0.0.0/33", "PRIVATE"] {
        let mut target = change();
        target.rule.cidr = cidr.into();
        cases.push(target);
    }
    for description in ["a".repeat(65), "private\nline".into()] {
        let mut target = change();
        target.rule.description = Some(description);
        cases.push(target);
    }
    for target in cases {
        assert!(
            request(Operation::Firewall(
                super::super::firewall::Operation::Create(target)
            ))
            .validate()
            .is_err()
        );
    }
    let mut target = change();
    target.rule.description = Some("not a delete selector".into());
    assert!(
        request(Operation::Firewall(
            super::super::firewall::Operation::Delete(target)
        ))
        .validate()
        .is_err()
    );
}

#[test]
fn families() {
    let mut body = response();
    body["FirewallRuleSet"][0]["Ipv6CidrBlock"] = Value::Null;
    let Data::Rules(page) = rules(0, 20).receipt(identity(), &body).unwrap().data else {
        panic!()
    };
    assert!(!page.complete);
    assert!(page.entries[0].ipv6.is_none());
    body["FirewallRuleSet"][0]["CidrBlock"] = "".into();
    body["FirewallRuleSet"][0]["Ipv6CidrBlock"] = "::/0".into();
    let Data::Rules(page) = rules(0, 20).receipt(identity(), &body).unwrap().data else {
        panic!()
    };
    assert!(page.complete);
    assert_eq!(page.entries[0].cidr.as_deref(), Some(""));
    assert_eq!(page.entries[0].ipv6.as_deref(), Some("::/0"));
    body["FirewallRuleSet"][0]["CidrBlock"] = "0.0.0.0/0".into();
    let Data::Rules(page) = rules(0, 20).receipt(identity(), &body).unwrap().data else {
        panic!()
    };
    assert!(!page.complete);
    for (cidr, ipv6) in [("192.0.2.1", ""), ("", "2001:db8::1")] {
        body["FirewallRuleSet"][0]["CidrBlock"] = cidr.into();
        body["FirewallRuleSet"][0]["Ipv6CidrBlock"] = ipv6.into();
        let Data::Rules(page) = rules(0, 20).receipt(identity(), &body).unwrap().data else {
            panic!()
        };
        assert!(page.complete);
    }
}
