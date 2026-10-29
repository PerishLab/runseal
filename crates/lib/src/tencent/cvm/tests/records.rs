use super::*;
use serde_json::{Value, json};

fn instance() -> Value {
    json!({"TotalCount":1,"InstanceSet":[{
        "InstanceId":"ins-fixture","InstanceName":"fixture","InstanceState":"PENDING",
        "Placement":{"Zone":"ap-hongkong-2","ProjectId":0},
        "VirtualPrivateCloud":{"VpcId":"vpc-fixture","SubnetId":"subnet-fixture"},
        "InternetAccessible":{"InternetChargeType":"TRAFFIC_POSTPAID_BY_HOUR","InternetMaxBandwidthOut":1},
        "InstanceType":"S5.SMALL2","ImageId":"img-fixture","InstanceChargeType":"POSTPAID_BY_HOUR",
        "SystemDisk":{"DiskId":"disk-system","DiskType":"CLOUD_PREMIUM","DiskSize":50},
        "DataDisks":[],"SecurityGroupIds":["sg-fixture"],"LoginSettings":{"KeyIds":["skey-fixture"],"Password":"PRIVATE_MARKER"},
        "DisableApiTermination":false,"PublicIpAddresses":null,"PrivateIpAddresses":["10.0.0.2"],
        "LatestOperation":"RunInstances","LatestOperationState":"OPERATING","LatestOperationRequestId":"creation-id",
        "LatestOperationErrorMsg":"PRIVATE_MARKER"
    }]})
}

#[test]
fn observation() {
    let get = request(Operation::Get {
        id: "ins-fixture".into(),
    });
    let mut value = instance();
    let Data::Instance(Some(found)) = get.decode(&value).unwrap() else {
        panic!("instance");
    };
    assert_eq!(found.progress.as_ref().unwrap().stage, Stage::Operating);
    assert!(found.facts.public.is_none());
    assert_eq!(found.facts.private.as_ref().unwrap().len(), 1);
    assert!(
        !serde_json::to_string(&found)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    for state in ["SUCCESS", "FAILED"] {
        value["InstanceSet"][0]["LatestOperationState"] = json!(state);
        assert!(get.decode(&value).is_ok());
    }
    for (field, invalid) in [
        ("LatestOperationState", json!("FUTURE")),
        ("LatestOperationRequestId", json!("unsafe\n")),
        ("InstanceId", json!("ins-foreign")),
        ("DataDisks", Value::Null),
    ] {
        let mut value = instance();
        value["InstanceSet"][0][field] = invalid;
        assert!(get.decode(&value).is_err());
    }
    let mut cross = instance();
    cross["InstanceSet"][0]["Placement"]["Zone"] = json!("ap-shanghai-2");
    assert!(get.decode(&cross).is_err());
    let mut missing = instance();
    missing["InstanceSet"][0]["LatestOperation"] = Value::Null;
    assert!(get.decode(&missing).is_err());
    let mut detached = instance();
    detached["InstanceSet"][0]["DataDisks"] = json!([{"DiskId":"disk-foreign","DiskType":"CLOUD_SSD","DiskSize":100,"DeleteWithInstance":true}]);
    let Data::Instance(Some(found)) = get.decode(&detached).unwrap() else {
        panic!("instance");
    };
    assert_eq!(found.facts.disks[0].id, "disk-foreign");
    assert_eq!(found.facts.disks[0].delete, Some(true));
    assert!(matches!(
        get.decode(&json!({"TotalCount":0,"InstanceSet":[]}))
            .unwrap(),
        Data::Instance(None)
    ));
    assert!(
        get.decode(&json!({"TotalCount":1,"InstanceSet":[]}))
            .is_err()
    );
    assert!(get.decode(&json!({"TotalCount":0})).is_err());
}

#[test]
fn status() {
    let get = request(Operation::Status {
        id: "ins-fixture".into(),
    });
    assert!(get.decode(&json!({"TotalCount":1,"InstanceStatusSet":[{"InstanceId":"ins-foreign","InstanceState":"RUNNING"}]})).is_err());
    let Data::Status(Some(found)) = get.decode(&json!({"TotalCount":1,"InstanceStatusSet":[{"InstanceId":"ins-fixture","InstanceState":"PENDING"}]})).unwrap() else { panic!("status"); };
    assert_eq!(found.state, "PENDING");
    assert!(matches!(
        get.decode(&json!({"TotalCount":0,"InstanceStatusSet":[]}))
            .unwrap(),
        Data::Status(None)
    ));
    let key = request(Operation::Key {
        id: "skey-fixture".into(),
    });
    let value = json!({"TotalCount":1,"KeyPairSet":[{"KeyId":"skey-fixture","KeyName":"fixture","ProjectId":0,"PublicKey":rsa(),"PrivateKey":"PRIVATE_MARKER","AssociatedInstanceIds":["ins-foreign"]}]});
    let Data::Key(Some(found)) = key.decode(&value).unwrap() else {
        panic!("key");
    };
    assert_eq!(found.instances, ["ins-foreign"]);
    assert!(
        !serde_json::to_string(&found)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let mut missing = value.clone();
    missing["KeyPairSet"][0]["AssociatedInstanceIds"] = Value::Null;
    assert!(key.decode(&missing).is_err());
    let mut wrong = value;
    wrong["KeyPairSet"][0]["KeyId"] = json!("skey-foreign");
    assert!(key.decode(&wrong).is_err());
    assert!(matches!(
        key.decode(&json!({"TotalCount":0,"KeyPairSet":[]}))
            .unwrap(),
        Data::Key(None)
    ));
}

#[test]
fn pricing() {
    let quote = request(Operation::Quote(spec()));
    let value = json!({"Price":{"InstancePrice":{"UnitPrice":0.2,"ChargeUnit":"HOUR","UnitPriceDiscount":0.1,"UnitPriceSecondStep":0.15,"UnitPriceDiscountSecondStep":0.08,"UnitPriceThirdStep":0.12},"BandwidthPrice":{"UnitPrice":0.8,"ChargeUnit":"GB"}}});
    let Data::Quote(price) = quote.decode(&value).unwrap() else {
        panic!("quote");
    };
    assert_eq!(price.instance.discounted, Some(0.1));
    assert_eq!(price.instance.second.as_ref().unwrap().price, Some(0.15));
    assert_eq!(price.instance.third.as_ref().unwrap().price, Some(0.12));
    assert!(price.currency.is_none());
    let mut unknown = value.clone();
    unknown["Price"]["InstancePrice"]["ChargeUnit"] = json!("MONTH");
    assert!(quote.decode(&unknown).is_err());
    let mut missing = value.clone();
    missing["Price"]["InstancePrice"]["UnitPrice"] = Value::Null;
    assert!(quote.decode(&missing).is_err());
    let mut negative = value;
    negative["Price"]["BandwidthPrice"]["UnitPrice"] = json!(-1);
    assert!(quote.decode(&negative).is_err());
    let quota = request(Operation::Quota {
        zone: Some("ap-hongkong-2".into()),
    });
    let rows = json!([{"Zone":"ap-hongkong-2","UsedQuota":2,"RemainingQuota":8,"TotalQuota":10}]);
    let value = json!({"AccountQuotaOverview":{"Region":"ap-hongkong","AccountQuota":{"PostPaidQuotaSet":rows}}});
    assert!(matches!(quota.decode(&value).unwrap(), Data::Quota(_)));
    let mut cross = value.clone();
    cross["AccountQuotaOverview"]["Region"] = json!("ap-shanghai");
    assert!(quota.decode(&cross).is_err());
    let mut unbounded = value;
    unbounded["AccountQuotaOverview"]["AccountQuota"]["PostPaidQuotaSet"][0]["RemainingQuota"] =
        json!(11);
    assert!(quota.decode(&unbounded).is_err());
    assert!(
        quota
            .decode(&json!({"AccountQuotaOverview":{"Region":"ap-hongkong","AccountQuota":{}}}))
            .is_err()
    );
}
