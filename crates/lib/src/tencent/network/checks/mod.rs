use super::{Data, Group, Id, Operation, Page, Request, Rule, Rules, Subnet, Vpc};
use crate::tencent::Class;
use serde_json::json;

pub(super) fn request(operation: Operation) -> Request {
    Request {
        region: "ap-hongkong".into(),
        operation,
    }
}

#[test]
fn exact() {
    let call = request(Operation::Vpc(Vpc::Get(Id {
        id: "vpc-fixture".into(),
    })))
    .call()
    .unwrap();
    assert_eq!(call.action, "DescribeVpcs");
    assert_eq!(
        call.body,
        json!({"VpcIds":["vpc-fixture"],"Limit":"1","Offset":"0"})
    );
    let data = call.decode(&json!({"TotalCount":1,"VpcSet":[{"VpcId":"vpc-fixture","VpcName":"fixture","CidrBlock":"10.42.0.0/16","IsDefault":false,"SecretKey":"SECRET_FIXTURE"}]})).unwrap();
    assert!(
        !serde_json::to_string(&data)
            .unwrap()
            .contains("SECRET_FIXTURE")
    );
    let Data::Entries { page } = data else {
        panic!("page");
    };
    assert_eq!(page.total, 1);
    assert_eq!(page.next, None);
    assert!(call.decode(&json!({"TotalCount":1,"VpcSet":[{"VpcId":"vpc-other","VpcName":"fixture","CidrBlock":"10.42.0.0/16"}]})).is_err());
    assert!(call.decode(&json!({"TotalCount":1,"VpcSet":[]})).is_err());
    assert!(call.decode(&json!({"VpcSet":[]})).is_err());
    let data = call.decode(&json!({"TotalCount":0,"VpcSet":[]})).unwrap();
    assert!(matches!(data, Data::Entries { .. }));
}

#[test]
fn pagination() {
    let call = request(Operation::Vpc(Vpc::List(Page {
        offset: 0,
        limit: 1,
    })))
    .call()
    .unwrap();
    let value = json!({"TotalCount":2,"VpcSet":[{"VpcId":"vpc-fixture","VpcName":"fixture","CidrBlock":"10.42.0.0/16"}]});
    let Data::Entries { page } = call.decode(&value).unwrap() else {
        panic!("page");
    };
    assert_eq!(page.next, Some(1));
    let mut invalid = value;
    invalid["TotalCount"] = json!(0);
    assert!(call.decode(&invalid).is_err());
    assert!(
        request(Operation::Vpc(Vpc::List(Page {
            offset: 0,
            limit: 0
        })))
        .call()
        .is_err()
    );
    assert!(
        request(Operation::Vpc(Vpc::List(Page {
            offset: 0,
            limit: 101
        })))
        .call()
        .is_err()
    );
}

#[test]
fn aggregate() {
    let rule = Rule::parse("tcp;443;0.0.0.0/0;accept").unwrap();
    let call = request(Operation::Group(Group::Reset {
        id: "sg-fixture".into(),
        rules: Rules {
            version: 12,
            ingress: vec![rule],
            egress: vec![],
        },
    }))
    .call()
    .unwrap();
    assert_eq!(call.action, "ModifySecurityGroupPolicies");
    assert_eq!(call.body["SecurityGroupPolicySet"]["Version"], "12");
    assert_eq!(
        call.body["SecurityGroupPolicySet"]["Ingress"][0]["Protocol"],
        "TCP"
    );
    assert_eq!(call.body["SecurityGroupPolicySet"]["Egress"], json!([]));
    assert_eq!(call.body["SortPolicys"], true);
    assert!(
        request(Operation::Group(Group::Reset {
            id: "sg-fixture".into(),
            rules: Rules {
                version: 0,
                ingress: vec![],
                egress: vec![]
            }
        }))
        .call()
        .is_err()
    );
    for value in [
        "tcp;-;0.0.0.0/0;accept",
        "icmp;443;0.0.0.0/0;accept",
        "all;-;MY_PUBLIC_IP;accept",
        "tcp;9000-8000;0.0.0.0/0;accept",
        "tcp;0;0.0.0.0/0;accept",
        "gre;-;0.0.0.0/0;accept",
    ] {
        assert!(Rule::parse(value).is_err());
    }
    assert!(Rule::parse("udp;53,100-120;10.0.0.0/8;drop").is_ok());
}

#[test]
fn policies() {
    let call = request(Operation::Group(Group::Rules(Id {
        id: "sg-fixture".into(),
    })))
    .call()
    .unwrap();
    let mut value = json!({"SecurityGroupPolicySet":{"Version":"2","Ingress":[{"Protocol":"TCP","Port":"443","CidrBlock":"0.0.0.0/0","Action":"ACCEPT"}],"Egress":[]}});
    assert!(call.decode(&value).is_ok());
    value["SecurityGroupPolicySet"]["Ingress"][0]["PolicyDescription"] =
        json!("foreign annotation");
    assert!(
        serde_json::to_string(&call.decode(&value).unwrap())
            .unwrap()
            .contains("foreign annotation")
    );
    value["SecurityGroupPolicySet"]["Ingress"][0]["PolicyDescription"] = json!("");
    value["SecurityGroupPolicySet"]["Ingress"][0]["AddressTemplate"] =
        json!({"AddressId":"ipm-fixture"});
    assert_eq!(call.decode(&value).unwrap_err().class, Class::Unsupported);
    value["SecurityGroupPolicySet"]["Egress"] = json!(null);
    assert!(call.decode(&value).is_err());
}

#[test]
fn local() {
    let mut invalid = request(Operation::Vpc(Vpc::Create {
        name: "network".into(),
        cidr: "10.42.0.0/16".into(),
    }));
    assert_eq!(invalid.call().unwrap().action, "CreateVpc");
    invalid.region = "ap--hongkong".into();
    assert!(invalid.call().is_err());
    assert!(
        request(Operation::Subnet(Subnet::Create {
            name: "private".into(),
            vpc: "vpc-fixture".into(),
            zone: "ap-singapore-1".into(),
            cidr: "10.42.1.0/24".into()
        }))
        .call()
        .is_err()
    );
    assert!(
        request(Operation::Vpc(Vpc::Delete(Id {
            id: "subnet-fixture".into()
        })))
        .call()
        .is_err()
    );
}

#[test]
fn metadata() {
    let call = request(Operation::Group(Group::Rules(Id {
        id: "sg-fixture".into(),
    })))
    .call()
    .unwrap();
    let rule = json!({"Protocol":"UDP","Port":"ALL","CidrBlock":"10.0.0.0/8","Action":"ACCEPT","AddressTemplate":{"AddressId":"","AddressGroupId":""},"ServiceTemplate":{},"PolicyDescription":"owned description"});
    let value = json!({"SecurityGroupPolicySet":{"Version":"1","Ingress":[rule],"Egress":[]}});
    let Data::Policies { ingress, .. } = call.decode(&value).unwrap() else {
        panic!("policies");
    };
    assert_eq!(ingress[0].ports.as_deref(), Some("all"));
    assert_eq!(ingress[0].description.as_deref(), Some("owned description"));
    let body = ingress[0].body().unwrap();
    assert_eq!(body["Port"], "ALL");
    assert_eq!(body["PolicyDescription"], "owned description");
    let call = request(Operation::Subnet(Subnet::Get(Id {
        id: "subnet-fixture".into(),
    })))
    .call()
    .unwrap();
    let data = call.decode(&json!({"TotalCount":1,"SubnetSet":[{"SubnetId":"subnet-fixture","VpcId":"vpc-fixture","SubnetName":"private","Zone":"ap-singapore-1","CidrBlock":"10.42.1.0/24"}]})).unwrap();
    assert!(data.regional("ap-hongkong").is_err());
}
