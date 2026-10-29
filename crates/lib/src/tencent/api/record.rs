use anyhow::{Result, bail};
use serde_json::Value;

pub(super) fn receipt(action: &str, value: &Value) -> Result<()> {
    let valid = match action {
        "CreateVpc" => value.get("Vpc").is_some_and(Value::is_object),
        "CreateSubnet" => value.get("Subnet").is_some_and(Value::is_object),
        "CreateSecurityGroup" => value.get("SecurityGroup").is_some_and(Value::is_object),
        "DescribeVpcs" => value.get("VpcSet").is_some_and(Value::is_array),
        "DescribeSubnets" => value.get("SubnetSet").is_some_and(Value::is_array),
        "DescribeSecurityGroups" => value.get("SecurityGroupSet").is_some_and(Value::is_array),
        "DescribeSecurityGroupPolicies" => value
            .get("SecurityGroupPolicySet")
            .is_some_and(Value::is_object),
        "ModifyVpcAttribute"
        | "ModifySubnetAttribute"
        | "ModifySecurityGroupAttribute"
        | "ModifySecurityGroupPolicies"
        | "DeleteVpc"
        | "DeleteSubnet"
        | "DeleteSecurityGroup" => true,
        "GetCallerIdentity" => value.get("AccountId").is_some_and(Value::is_string),
        "GetUser" | "AddUser" => value
            .get("Uin")
            .is_some_and(|held| held.is_u64() || held.is_string()),
        "ListPolicies" => value
            .get("List")
            .is_some_and(|held| held.is_array() || held.is_null()),
        "GetPolicy" => value.get("PolicyDocument").is_some_and(Value::is_string),
        "CreatePolicy" => value.get("PolicyId").is_some_and(Value::is_u64),
        "ListAccessKeys" => value
            .get("AccessKeys")
            .is_some_and(|held| held.is_array() || held.is_null()),
        "CreateAccessKey" => value.get("AccessKey").is_some_and(Value::is_object),
        "DescribeDomain" => value.get("DomainInfo").is_some_and(Value::is_object),
        "DescribeRecordList" => value.get("RecordList").is_some_and(Value::is_array),
        "CreateRecord" | "ModifyRecord" => value.get("RecordId").is_some_and(Value::is_u64),
        "DescribeInstances" => value.get("InstanceSet").is_some_and(Value::is_array),
        "DescribeRegions" => value.get("RegionSet").is_some_and(Value::is_array),
        "DescribeZones" => ["ZoneSet", "ZoneInfoSet"]
            .iter()
            .any(|key| value.get(key).is_some_and(Value::is_array)),
        "DescribeZoneInstanceConfigInfos" => value
            .get("InstanceTypeQuotaSet")
            .is_some_and(Value::is_array),
        "DescribeImages" => value.get("ImageSet").is_some_and(Value::is_array),
        "DescribeBlueprints" => value.get("BlueprintSet").is_some_and(Value::is_array),
        "DescribeBundles" => value.get("BundleSet").is_some_and(Value::is_array),
        "AttachUserPolicy" | "DetachUserPolicy" | "UpdateAccessKey" | "DeleteAccessKey"
        | "DeleteRecord" => true,
        _ => false,
    };
    if !valid {
        bail!(
            "Tencent operation receipt incomplete; a mutation may have completed; inspect the exact resource before recovery"
        );
    }
    Ok(())
}

const FIELDS: &[&str] = &[
    "RequestId",
    "AccountId",
    "UserId",
    "PrincipalId",
    "Type",
    "Arn",
    "Uin",
    "Uid",
    "Name",
    "Remark",
    "ConsoleLogin",
    "UseApi",
    "PolicyId",
    "PolicyName",
    "Description",
    "PolicyDocument",
    "PolicyType",
    "List",
    "TotalNum",
    "AccessKeyId",
    "AccessKeys",
    "Status",
    "CreateTime",
    "RecordId",
    "RecordList",
    "RecordCountInfo",
    "TotalCount",
    "SubdomainCount",
    "ListCount",
    "DomainInfo",
    "Id",
    "Domain",
    "TTL",
    "Line",
    "LineId",
    "Value",
    "MX",
    "UpdatedOn",
    "MonitorStatus",
    "DefaultNS",
    "IsMark",
    "IsPaused",
    "Grade",
    "GradeLevel",
    "GradeTitle",
    "IsVip",
    "Owner",
    "Records",
    "IsPendingShare",
    "Punycode",
    "RouteId",
    "DomainId",
    "ValueCount",
];

pub(in crate::tencent) fn metadata(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(name, _)| FIELDS.contains(&name.as_str()))
                .map(|(name, held)| (name.clone(), metadata(held)))
                .collect::<serde_json::Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.iter().take(200).map(metadata).collect()),
        Value::String(text) if text.len() > 128 * 1024 => Value::Null,
        other => other.clone(),
    }
}
