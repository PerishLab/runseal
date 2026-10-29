use super::{Dns, Fields, Record};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(in crate::tencent) fn request(deed: &Dns) -> Result<(&'static str, Value, bool)> {
    let found = match deed {
        Dns::Domain { domain } => ("DescribeDomain", json!({"Domain": domain}), false),
        Dns::Record(Record::List { domain, offset }) => (
            "DescribeRecordList",
            json!({"Domain": domain, "Offset": offset, "Limit": 100}),
            false,
        ),
        Dns::Record(Record::Create(record)) => ("CreateRecord", fields(record)?, true),
        Dns::Record(Record::Modify { id, record }) => {
            let mut value = fields(record)?;
            value["RecordId"] = positive(*id)?;
            ("ModifyRecord", value, true)
        }
        Dns::Record(Record::Delete { domain, id }) => (
            "DeleteRecord",
            json!({"Domain": domain, "RecordId": positive(*id)?}),
            true,
        ),
    };
    let domain = found
        .1
        .get("Domain")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if domain.len() > 253
        || !domain.contains('.')
        || !domain
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
    {
        bail!("Tencent domain refused; supply an explicit ASCII zone name");
    }
    Ok(found)
}

fn fields(record: &Fields) -> Result<Value> {
    let name = (1..=253).contains(&record.name.len());
    let value = (1..=4096).contains(&record.value.len());
    if !name || !value || record.ttl == 0 {
        bail!("Tencent DNS record refused");
    }
    match record.kind.as_str() {
        "A" => {
            let _: std::net::Ipv4Addr = record
                .value
                .parse()
                .map_err(|_| anyhow::anyhow!("Tencent IPv4 record refused"))?;
        }
        "AAAA" => {
            let _: std::net::Ipv6Addr = record
                .value
                .parse()
                .map_err(|_| anyhow::anyhow!("Tencent IPv6 record refused"))?;
        }
        _ => (),
    }
    Ok(
        json!({"Domain": record.domain, "SubDomain": record.name, "RecordType": record.kind, "RecordLine": record.line, "Value": record.value, "TTL": record.ttl}),
    )
}

pub(super) fn positive(value: u64) -> Result<Value> {
    if value == 0 {
        bail!("Tencent resource identifier must be positive");
    }
    Ok(json!(value))
}
