mod record;
mod types;

use super::{
    Data, Page as Window, input,
    names::Names,
    request::{Call, Kind},
};
use crate::tencent::{Class, Failure, Page};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub use types::{Address, Attachment, Interface, Location};

pub(super) fn call(instance: &str, page: Window) -> Result<Call, Failure> {
    Names::id(instance, "ins-")?;
    if !(1..=100).contains(&page.limit) || page.offset.checked_add(page.limit).is_none() {
        return Err(input());
    }
    Ok(Call {
        action: "DescribeNetworkInterfaces",
        body: json!({"Filters":[{"Name":"attachment.instance-id","Values":[instance]}],"Offset":page.offset,"Limit":page.limit}),
        kind: Kind::Interfaces(page),
    })
}

pub(super) fn decode(request: &Value, value: &Value, window: Window) -> Result<Data, Failure> {
    let instance = request["Filters"][0]["Values"][0]
        .as_str()
        .ok_or_else(response)?;
    let values = value["NetworkInterfaceSet"]
        .as_array()
        .ok_or_else(response)?;
    let total = value["TotalCount"].as_u64().ok_or_else(response)?;
    let count = u32::try_from(values.len()).map_err(|_| response())?;
    let end = window.offset.checked_add(count).ok_or_else(response)?;
    if count > window.limit {
        return Err(response());
    }
    match (count, total.cmp(&u64::from(end))) {
        (0, std::cmp::Ordering::Greater) | (1.., std::cmp::Ordering::Less) => {
            return Err(response());
        }
        _ => (),
    }
    let mut ids = BTreeSet::new();
    let mut items = Vec::new();
    for value in values {
        let entry = record::Record(value).interface(instance)?;
        if !ids.insert(entry.id.clone()) {
            return Err(response());
        }
        items.push(entry);
    }
    Ok(Data::Interfaces {
        page: Page {
            items,
            total,
            offset: window.offset,
            limit: window.limit,
            next: (count > 0 && u64::from(end) < total).then_some(end),
        },
    })
}

pub(super) fn regional(entry: &Interface, region: &str) -> Result<(), Failure> {
    if let Some(zone) = &entry.location.zone {
        let prefix = format!("{region}-");
        let suffix = zone
            .strip_prefix(&prefix)
            .filter(|s| !s.is_empty())
            .ok_or_else(response)?;
        if !suffix.bytes().all(|b| b.is_ascii_digit()) {
            return Err(response());
        }
    }
    Ok(())
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "network interface observation is incomplete, inconsistent or outside the finite model",
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod transport;
