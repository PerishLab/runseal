use super::*;
use crate::tencent::{
    command::Deed,
    network::{Operation, Request},
    parse,
};

fn request() -> Call {
    call(
        "ins-fixture",
        Window {
            offset: 0,
            limit: 2,
        },
    )
    .unwrap()
}
fn row() -> Value {
    json!({"NetworkInterfaceId":"eni-fixture","VpcId":"vpc-fixture","SubnetId":"subnet-fixture","Zone":"ap-hongkong-2","Primary":true,"EniType":1,"Business":"cvm","State":"AVAILABLE","NetworkInterfaceState":"INUSE","TerminationProtection":false,"TrafficProtection":false,"AttachType":0,"ResourceId":"","GroupSet":["sg-fixture"],"Attachment":{"InstanceId":"ins-fixture","InstanceAccountId":"100","DeviceIndex":0},"PrivateIpAddressSet":[{"PrivateIpAddress":"10.42.1.8","Primary":true,"PublicIpAddress":"203.0.113.8","AddressId":"","State":"AVAILABLE","IsWanIpBlocked":false}],"Ipv6AddressSet":[],"TagSet":[{"Key":"private","Value":"WITHHELD_FIXTURE"}],"NetworkInterfaceDescription":"WITHHELD_FIXTURE"})
}
fn payload(row: Value) -> Value {
    json!({"TotalCount":1,"NetworkInterfaceSet":[row]})
}
fn page(value: &Value) -> Page<Interface> {
    let Data::Interfaces { page } = request().decode(value).unwrap() else {
        panic!("interface page")
    };
    page
}

#[test]
fn mapping() {
    let request = request();
    assert_eq!(request.action, "DescribeNetworkInterfaces");
    assert!(!request.mutation());
    assert_eq!(
        request.body,
        json!({"Filters":[{"Name":"attachment.instance-id","Values":["ins-fixture"]}],"Offset":0,"Limit":2})
    );
    assert!(
        call(
            "vpc-foreign",
            Window {
                offset: 0,
                limit: 1
            }
        )
        .is_err()
    );
    assert!(
        call(
            "ins-fixture",
            Window {
                offset: 0,
                limit: 101
            }
        )
        .is_err()
    );
    assert!(
        call(
            "ins-fixture",
            Window {
                offset: 0,
                limit: 0
            }
        )
        .is_err()
    );
    assert!(
        call(
            "ins-fixture",
            Window {
                offset: u32::MAX,
                limit: 2
            }
        )
        .is_err()
    );
    let argv = [
        "network",
        "--region",
        "ap-hongkong",
        "interfaces",
        "--instance",
        "ins-fixture",
        "--limit",
        "2",
    ]
    .map(str::to_owned);
    let command = parse(&argv).unwrap();
    let Deed::Network(request) = command.deed else {
        panic!("network")
    };
    request.validate().unwrap();
    assert!(matches!(
        request.operation,
        Operation::Interfaces {
            page: Window { limit: 2, .. },
            ..
        }
    ));
    let mut raw = argv.to_vec();
    raw.extend(["--action".into(), "DeleteNetworkInterface".into()]);
    assert!(parse(&raw).is_err());
    let bad = Request {
        region: "foreign".into(),
        operation: request.operation,
    };
    assert!(bad.validate().is_err());
}

#[test]
fn facts() {
    let page = page(&payload(row()));
    assert_eq!(page.total, 1);
    assert_eq!(page.next, None);
    let entry = &page.items[0];
    assert_eq!(entry.id, "eni-fixture");
    assert_eq!(entry.attachment.instance, "ins-fixture");
    assert_eq!(entry.attachment.owner.as_deref(), Some("100"));
    assert_eq!(entry.attachment.device, Some(0));
    assert_eq!(entry.primary, Some(true));
    assert!(entry.coverage);
    assert_eq!(entry.ipv6.as_ref().unwrap().len(), 0);
    let address = &entry.ipv4.as_ref().unwrap()[0];
    assert_eq!(address.elastic.as_deref(), Some(""));
    assert_eq!(address.public.as_deref(), Some("203.0.113.8"));
    assert_eq!(address.ip.to_string(), "10.42.1.8");
    assert!(address.coverage);
    let serialized = serde_json::to_string(entry).unwrap();
    assert!(!serialized.contains("WITHHELD_FIXTURE"));
    assert!(!serialized.contains("NetworkInterfaceDescription"));
    regional(entry, "ap-hongkong").unwrap();
    assert!(regional(entry, "ap-guangzhou").is_err());
}

#[test]
fn unknown() {
    let mut data = row();
    data["PrivateIpAddressSet"][0]
        .as_object_mut()
        .unwrap()
        .remove("AddressId");
    data["PrivateIpAddressSet"][0]["PublicIpAddress"] = Value::Null;
    data["Attachment"]["InstanceAccountId"] = Value::Null;
    data.as_object_mut().unwrap().remove("Primary");
    data["Ipv6AddressSet"] = Value::Null;
    let unknown = page(&payload(data));
    let entry = &unknown.items[0];
    assert!(entry.primary.is_none());
    assert!(entry.attachment.owner.is_none());
    assert!(entry.ipv6.is_none());
    let address = &entry.ipv4.as_ref().unwrap()[0];
    assert!(address.elastic.is_none());
    assert!(address.public.is_none());
    let mut empty = row();
    empty["PrivateIpAddressSet"][0]["PublicIpAddress"] = json!("");
    assert_eq!(page(&payload(empty)).items.len(), 1);
}

#[test]
fn foreign() {
    let mut data = row();
    data["Attachment"]["InstanceId"] = json!("ins-other");
    assert!(request().decode(&payload(data)).is_err());
    let mut data = row();
    data["Attachment"] = Value::Null;
    assert!(request().decode(&payload(data)).is_err());
    let mut data = row();
    data["Attachment"]["InstanceAccountId"] = json!("999");
    data["Primary"] = json!(false);
    data["PrivateIpAddressSet"][0]["AddressId"] = json!("eip-foreign");
    data["Ipv6AddressSet"] =
        json!([{"Address":"2001:db8::8","Primary":false,"AddressId":"eip-ipv6"}]);
    let page = page(&payload(data));
    let entry = &page.items[0];
    assert_eq!(entry.attachment.owner.as_deref(), Some("999"));
    assert_eq!(entry.primary, Some(false));
    assert_eq!(
        entry.ipv4.as_ref().unwrap()[0].elastic.as_deref(),
        Some("eip-foreign")
    );
    assert_eq!(
        entry.ipv6.as_ref().unwrap()[0].elastic.as_deref(),
        Some("eip-ipv6")
    );
}

#[test]
fn coverage() {
    let mut data = row();
    data["FutureAttachment"] = json!({"Secret":"WITHHELD_FIXTURE"});
    let result = page(&payload(data));
    assert!(!result.items[0].coverage);
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("WITHHELD_FIXTURE")
    );
    let mut data = row();
    data["PrivateIpAddressSet"][0]["FutureAddress"] = json!(1);
    assert!(!page(&payload(data)).items[0].ipv4.as_ref().unwrap()[0].coverage);
    let mut data = row();
    data["GroupSet"] = json!(["sg-fixture", "sg-fixture"]);
    assert!(request().decode(&payload(data)).is_err());
    let mut data = row();
    data["PrivateIpAddressSet"][0]["AddressId"] = json!("untrusted-body");
    assert!(request().decode(&payload(data)).is_err());
}

#[test]
fn pagination() {
    let mut value = payload(row());
    value["TotalCount"] = json!(3);
    assert_eq!(page(&value).next, Some(1));
    let mut duplicate = value.clone();
    duplicate["NetworkInterfaceSet"] = json!([row(), row()]);
    assert!(request().decode(&duplicate).is_err());
    let empty = json!({"TotalCount":0,"NetworkInterfaceSet":[]});
    assert!(page(&empty).items.is_empty());
    assert!(
        request()
            .decode(&json!({"TotalCount":1,"NetworkInterfaceSet":[]}))
            .is_err()
    );
    value["TotalCount"] = json!(0);
    assert!(request().decode(&value).is_err());
    assert!(request().decode(&json!({"TotalCount":1})).is_err());
    let single = call(
        "ins-fixture",
        Window {
            offset: 0,
            limit: 1,
        },
    )
    .unwrap();
    let mut other = row();
    other["NetworkInterfaceId"] = json!("eni-other");
    assert!(
        single
            .decode(&json!({"TotalCount":2,"NetworkInterfaceSet":[row(),other]}))
            .is_err()
    );
}
