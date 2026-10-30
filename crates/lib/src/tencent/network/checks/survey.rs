use super::{Data, Group, Id, Operation, Subnet, Survey, Vpc, survey::Probe, tests::request};
use serde_json::{Value, json};

pub(super) fn call(probe: Probe) -> super::request::Call {
    match probe {
        Probe::Vpc => request(Operation::Vpc(Vpc::Survey(Id {
            id: "vpc-fixture".into(),
        }))),
        Probe::Subnet => request(Operation::Subnet(Subnet::Survey(Id {
            id: "subnet-fixture".into(),
        }))),
        Probe::Group => request(Operation::Group(Group::Associations(Id {
            id: "sg-fixture".into(),
        }))),
    }
    .call()
    .unwrap()
}

fn vpc() -> Value {
    let mut row = json!({"VpcId":"vpc-fixture", "SubnetId":""});
    for field in [
        "Classiclink",
        "Dcg",
        "Pcx",
        "Ip",
        "Nat",
        "Vpngw",
        "FlowLog",
        "NetworkDetect",
        "NetworkACL",
        "CVM",
        "LB",
        "CDB",
        "Cmem",
        "CTSDB",
        "MariaDB",
        "SQLServer",
        "Postgres",
        "NAS",
        "Greenplumn",
        "Ckafka",
        "Grocery",
        "HSM",
        "Tcaplus",
        "Cnas",
        "TiDB",
        "Emr",
        "SEAL",
        "CFS",
        "Oracle",
        "ElasticSearch",
        "TBaaS",
        "Itop",
        "DBAudit",
        "CynosDBPostgres",
        "Redis",
        "MongoDB",
        "DCDB",
        "CynosDBMySQL",
        "Subnet",
        "RouteTable",
    ] {
        row[field] = json!(0);
    }
    json!({"ResourceDashboardSet":[row]})
}

fn read(call: &super::request::Call, value: &Value) -> Survey {
    let Data::Survey { entry: Some(entry) } = call.decode(value).unwrap() else {
        panic!("survey")
    };
    entry
}

#[test]
fn requests() {
    struct Case {
        probe: Probe,
        action: &'static str,
        field: &'static str,
        id: &'static str,
    }
    for case in [
        Case {
            probe: Probe::Vpc,
            action: "DescribeVpcResourceDashboard",
            field: "VpcIds",
            id: "vpc-fixture",
        },
        Case {
            probe: Probe::Subnet,
            action: "DescribeSubnetResourceDashboard",
            field: "SubnetIds",
            id: "subnet-fixture",
        },
        Case {
            probe: Probe::Group,
            action: "DescribeSecurityGroupAssociationStatistics",
            field: "SecurityGroupIds",
            id: "sg-fixture",
        },
    ] {
        let call = call(case.probe);
        assert_eq!(call.action, case.action);
        assert_eq!(call.body, json!({case.field:[case.id]}));
        assert!(!call.mutation());
        assert!(case.probe.call("wrong-kind").is_err());
    }
}

#[test]
fn coverage() {
    let call = call(Probe::Vpc);
    let mut value = vpc();
    let entry = read(&call, &value);
    assert!(entry.complete);
    assert_eq!(entry.counts.len(), 40);
    value["ResourceDashboardSet"][0]["Ip"] = Value::Null;
    let entry = read(&call, &value);
    assert!(!entry.complete);
    assert_eq!(
        entry
            .counts
            .iter()
            .find(|v| v.name.as_deref() == Some("Ip"))
            .unwrap()
            .value,
        None
    );
    value["ResourceDashboardSet"][0]
        .as_object_mut()
        .unwrap()
        .remove("Nat");
    assert!(!read(&call, &value).complete);
    value["ResourceDashboardSet"][0]["SECRET_FIXTURE"] = json!("PRIVATE_MARKER");
    let output = serde_json::to_string(&read(&call, &value)).unwrap();
    assert!(!output.contains("SECRET_FIXTURE"));
    assert!(!output.contains("PRIVATE_MARKER"));
    value["ResourceDashboardSet"][0]["Ip"] = json!(-1);
    assert!(call.decode(&value).is_err());
    value["ResourceDashboardSet"][0]["Ip"] = json!(1.5);
    assert!(call.decode(&value).is_err());
    value["ResourceDashboardSet"][0]["Ip"] = json!("0");
    assert!(call.decode(&value).is_err());
}

#[test]
fn identity() {
    let call = call(Probe::Vpc);
    assert!(call.decode(&json!({})).is_err());
    assert!(matches!(
        call.decode(&json!({"ResourceDashboardSet":[]})).unwrap(),
        Data::Survey { entry: None }
    ));
    let mut value = vpc();
    value["ResourceDashboardSet"][0]["VpcId"] = json!("vpc-foreign");
    assert!(call.decode(&value).is_err());
    value = vpc();
    let row = value["ResourceDashboardSet"][0].clone();
    value["ResourceDashboardSet"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert!(call.decode(&value).is_err());
    value = vpc();
    value["ResourceDashboardSet"][0]["SubnetId"] = json!("subnet-foreign");
    assert!(call.decode(&value).is_err());
}

#[test]
fn subnet() {
    let call = call(Probe::Subnet);
    let mut value = json!({"ResourceStatisticsSet":[{"VpcId":"vpc-fixture","SubnetId":"subnet-fixture","Ip":1,"ResourceStatisticsItemSet":[{"ResourceType":"ENI","ResourceName":"公开显示名","ResourceCount":1}]}]});
    let entry = read(&call, &value);
    assert!(entry.complete);
    assert_eq!(entry.parent.as_deref(), Some("vpc-fixture"));
    assert_eq!(entry.instances.unwrap()[0].value, Some(1));
    value["ResourceStatisticsSet"][0]["ResourceStatisticsItemSet"][0]["ResourceType"] =
        json!("FUTURE_SECRET_MARKER");
    let entry = read(&call, &value);
    assert!(!entry.complete);
    assert!(entry.instances.as_ref().unwrap()[0].name.is_none());
    assert!(
        !serde_json::to_string(&entry)
            .unwrap()
            .contains("FUTURE_SECRET_MARKER")
    );
    value["ResourceStatisticsSet"][0]["ResourceStatisticsItemSet"] = Value::Null;
    assert!(read(&call, &value).instances.is_none());
    value["ResourceStatisticsSet"][0]["VpcId"] = Value::Null;
    assert!(!read(&call, &value).complete);
    value["ResourceStatisticsSet"][0]["VpcId"] = json!("ins-wrong");
    assert!(call.decode(&value).is_err());
}

#[test]
fn associations() {
    let call = call(Probe::Group);
    let mut value = json!({"SecurityGroupAssociationStatisticsSet":[{"SecurityGroupId":"sg-fixture","CVM":1,"CDB":1,"ENI":1,"CLB":1,"SG":1,"TotalCount":1,"InstanceStatistics":[{"InstanceType":"CVM","InstanceCount":1}]}]});
    assert!(read(&call, &value).complete);
    let row = value["SecurityGroupAssociationStatisticsSet"][0]["InstanceStatistics"][0].clone();
    value["SecurityGroupAssociationStatisticsSet"][0]["InstanceStatistics"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert!(call.decode(&value).is_err());
    value["SecurityGroupAssociationStatisticsSet"][0]["InstanceStatistics"] = json!([]);
    value["SecurityGroupAssociationStatisticsSet"][0]["SG"] = Value::Null;
    assert!(!read(&call, &value).complete);
}

#[test]
fn native() {
    for (kind, verb) in [
        ("vpc", "survey"),
        ("subnet", "survey"),
        ("group", "associations"),
    ] {
        let id = match kind {
            "vpc" => "vpc-fixture",
            "subnet" => "subnet-fixture",
            _ => "sg-fixture",
        };
        let argv: Vec<String> = ["network", "--region", "ap-hongkong", kind, verb, "--id", id]
            .into_iter()
            .map(Into::into)
            .collect();
        let parsed = crate::tencent::parse(&argv).unwrap();
        let crate::tencent::command::Deed::Network(request) = parsed.deed else {
            panic!("network")
        };
        assert!(!request.call().unwrap().mutation());
        let mut bad = argv;
        bad.extend(["--raw-body".into(), "{}".into()]);
        assert!(crate::tencent::parse(&bad).is_err());
    }
}
