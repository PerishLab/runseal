use super::*;

pub(super) fn instance() -> Value {
    json!({"InstanceId":"lhins-fixture","Uuid":"fixture-uuid","CreatedTime":"2026-11-02T00:00:00Z",
        "Zone":"na-ashburn-1","InstanceState":"RUNNING","PlatformType":"LINUX_UNIX",
        "PublicAddresses":["192.0.2.1"],"PrivateAddresses":["10.0.0.1"],"PublicIpv6Addresses":["2001:db8::1"],
        "LoginSettings":{"KeyIds":["lhkp-fixture"]},"LatestOperation":"AssociateInstancesKeyPairs",
        "LatestOperationState":"SUCCESS","LatestOperationRequestId":"operation-id","LatestOperationStartedTime":"2026-11-02T00:00:01Z"})
}

pub(super) fn key() -> Value {
    json!({"KeyId":"lhkp-fixture","KeyName":"public","PublicKey":"",
        "AssociatedInstanceIds":["lhins-fixture"],"AssociatedInstanceSet":[
        {"InstanceId":"lhins-fixture","Username":"root"},{"InstanceId":"lhins-fixture","Username":"ubuntu"}],
        "CreatedTime":"2026-11-02T00:00:00Z","PrivateKey":"PRIVATE_MARKER","Tags":[{"Value":"PRIVATE_MARKER"}]})
}

pub(super) fn page(field: &str, value: Value) -> Value {
    json!({field:[value],"TotalCount":1,"RequestId":"fixture-id"})
}
