use super::*;
use serde_json::{Value, json};

mod records;
mod wire;

fn request(operation: Operation) -> Request {
    Request {
        region: "na-ashburn".into(),
        operation,
    }
}

fn agent() -> Request {
    request(Operation::Lighthouse {
        id: "lhins-fixture".into(),
    })
}

fn facts() -> Value {
    json!({"InstanceId":"lhins-fixture","Version":"1.0.0", "LastHeartbeatTime":"2026-11-02T02:00:00Z",
        "AgentStatus":"Online","Environment":"Linux","SupportFeatures":["SESSION_MANAGER"]})
}

fn page(value: Value) -> Value {
    json!({"AutomationAgentSet":[value],"TotalCount":1,"RequestId":"fixture-id"})
}

#[test]
fn requests() {
    for (operation, action, body) in [
        (Operation::Regions, "DescribeRegions", json!({})),
        (
            Operation::Lighthouse {
                id: "lhins-fixture".into(),
            },
            "DescribeAutomationAgentStatus",
            json!({"InstanceIds":["lhins-fixture"],"Offset":0,"Limit":1}),
        ),
    ] {
        let request = request(operation);
        let call = request.call().unwrap();
        let native = call.request(&request.region);
        assert_eq!(native.service, "tat");
        assert_eq!(native.region, Some("na-ashburn"));
        assert_eq!(native.action, action);
        assert_eq!(*native.params, body);
    }
    for region in [
        "",
        "na-ashburn-1",
        "na-ashburn\n",
        "https://foreign",
        "NA-ashburn",
    ] {
        let mut input = agent();
        input.region = region.into();
        assert_eq!(input.validate().unwrap_err().class, Class::Input);
    }
    for id in ["ins-fixture", "lhins-", "lhins-bad_name", "lhins-fixture\n"] {
        assert!(
            request(Operation::Lighthouse { id: id.into() })
                .validate()
                .is_err()
        );
    }
}

#[test]
fn grammar() {
    for verb in [vec!["regions"], vec!["agent", "--id", "lhins-fixture"]] {
        let mut words = vec!["tat", "--region", "na-ashburn"];
        words.extend(verb);
        let input =
            crate::tencent::parse(&words.into_iter().map(str::to_owned).collect::<Vec<_>>())
                .unwrap();
        let crate::tencent::command::Deed::Tat(deed) = input.deed else {
            panic!("TAT");
        };
        let request: Request = (&deed).into();
        assert_eq!(request.region, "na-ashburn");
        request.validate().unwrap();
    }
    for words in [
        vec!["tat", "regions"],
        vec!["tat", "--region", "na-ashburn", "run-command"],
    ] {
        assert!(
            crate::tencent::parse(&words.into_iter().map(str::to_owned).collect::<Vec<_>>())
                .is_err()
        );
    }
}

#[cfg(unix)]
mod custody;
