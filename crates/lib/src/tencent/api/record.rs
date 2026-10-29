use serde_json::Value;

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
