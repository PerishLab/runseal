use super::*;

#[test]
fn observations() {
    let Data::Agent {
        id,
        value,
        total,
        complete,
    } = agent().decode(&page(facts())).unwrap()
    else {
        panic!("agent");
    };
    assert_eq!(id, "lhins-fixture");
    assert_eq!(total, 1);
    assert!(complete);
    let value = value.unwrap();
    assert!(matches!(value.status, Some(Status::Online)));
    assert_eq!(value.heartbeat.as_deref(), Some("2026-11-02T02:00:00Z"));
    assert!(matches!(
        value.features.as_ref().unwrap()[0],
        Feature::Session
    ));
    let data = agent()
        .decode(&json!({"AutomationAgentSet":[],"TotalCount":0}))
        .unwrap();
    assert!(matches!(
        data,
        Data::Agent {
            value: None,
            total: 0,
            ..
        }
    ));
    for label in ["Offline", "PRIVATE_MARKER"] {
        let mut value = facts();
        value["AgentStatus"] = json!(label);
        value["Environment"] = json!("PRIVATE_MARKER");
        value["SupportFeatures"] = json!(["PRIVATE_MARKER"]);
        value["SecretKey"] = json!("PRIVATE_MARKER");
        let output = serde_json::to_value(agent().decode(&page(value)).unwrap()).unwrap();
        assert_eq!(output["complete"], false);
        assert_eq!(
            output["value"]["status"],
            if label == "Offline" {
                "offline"
            } else {
                "unknown"
            }
        );
        assert!(!output.to_string().contains("PRIVATE_MARKER"));
    }
}

#[test]
fn nullable() {
    for field in [
        "Version",
        "LastHeartbeatTime",
        "AgentStatus",
        "Environment",
        "SupportFeatures",
    ] {
        for present in [false, true] {
            let mut value = facts();
            if present {
                value[field] = Value::Null;
            } else {
                value.as_object_mut().unwrap().remove(field);
            }
            let Data::Agent {
                value: Some(value), ..
            } = agent().decode(&page(value)).unwrap()
            else {
                panic!("facts");
            };
            match field {
                "Version" => assert!(value.version.is_none()),
                "LastHeartbeatTime" => assert!(value.heartbeat.is_none()),
                "AgentStatus" => assert!(value.status.is_none()),
                "Environment" => assert!(value.environment.is_none()),
                _ => assert!(value.features.is_none()),
            }
        }
    }
    let mut value = facts();
    value["SupportFeatures"] = json!([]);
    let Data::Agent {
        value: Some(value), ..
    } = agent().decode(&page(value)).unwrap()
    else {
        panic!("facts");
    };
    assert!(value.features.unwrap().is_empty());
}

#[test]
fn conflicts() {
    for body in [
        json!({}),
        json!({"TotalCount":0}),
        json!({"AutomationAgentSet":null,"TotalCount":0}),
        json!({"AutomationAgentSet":[],"TotalCount":1}),
        json!({"AutomationAgentSet":[],"TotalCount":null}),
        json!({"AutomationAgentSet":[facts(),facts()],"TotalCount":2}),
    ] {
        assert!(agent().decode(&body).is_err());
    }
    for (field, value) in [
        ("InstanceId", json!("lhins-foreign")),
        ("InstanceId", Value::Null),
        ("Version", json!({"SecretKey":"PRIVATE_MARKER"})),
        ("LastHeartbeatTime", json!("invalid")),
        ("AgentStatus", json!(42)),
        (
            "SupportFeatures",
            json!(["SESSION_MANAGER", "SESSION_MANAGER"]),
        ),
    ] {
        let mut body = facts();
        body[field] = value;
        let error = agent().decode(&page(body)).unwrap_err();
        assert_eq!(error.class, Class::Response);
        assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
    }
}

#[test]
fn regions() {
    let request = request(Operation::Regions);
    let value =
        json!({"Region":"na-ashburn","RegionName":"PRIVATE_MARKER","RegionState":"AVAILABLE"});
    let body = json!({"RegionSet":[value],"TotalCount":1,"RequestId":"fixture-id"});
    let result = serde_json::to_value(request.decode(&body).unwrap()).unwrap();
    assert_eq!(result["entries"][0]["state"], "available");
    assert_eq!(result["complete"], true);
    assert!(!result.to_string().contains("PRIVATE_MARKER"));
    for state in [Value::Null, json!("UNAVAILABLE"), json!("PRIVATE_MARKER")] {
        let mut body = body.clone();
        body["RegionSet"][0]["RegionState"] = state;
        assert!(
            !serde_json::to_string(&request.decode(&body).unwrap())
                .unwrap()
                .contains("PRIVATE_MARKER")
        );
    }
    for bad in [
        json!({"RegionSet":[value,value],"TotalCount":2}),
        json!({"RegionSet":[],"TotalCount":1}),
        json!({"RegionSet":null,"TotalCount":0}),
        json!({"RegionSet":[],"TotalCount":-1}),
    ] {
        assert!(request.decode(&bad).is_err());
    }
}

#[test]
fn receipts() {
    let identity = crate::tencent::Identity {
        account: "123".into(),
        principal: "456".into(),
        kind: "CAMUser".into(),
        stamp: "fixture-stamp".into(),
        observed: 1,
        request: "identity-id".into(),
    };
    let receipt = agent().receipt(identity.clone(), &page(facts())).unwrap();
    assert_eq!(receipt.identity.account, "123");
    assert_eq!(receipt.identity.principal, "456");
    assert_eq!(receipt.region, "na-ashburn");
    assert_eq!(receipt.service, "tat");
    assert_eq!(receipt.version, "2020-10-28");
    assert_eq!(receipt.request, "fixture-id");
    assert_eq!(receipt.outcome, Outcome::Observed);
    let error = agent()
        .receipt(identity.clone(), &page(json!({"InstanceId":"foreign"})))
        .unwrap_err();
    assert_eq!(error.request.as_deref(), Some("fixture-id"));
    let mut body = page(facts());
    body["RequestId"] = json!("PRIVATE_MARKER\n");
    let error = agent().receipt(identity, &body).unwrap_err();
    assert!(error.request.is_none());
    assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
}
