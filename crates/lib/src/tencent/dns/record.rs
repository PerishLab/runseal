use super::{Data, Domain, Entry, Operation, Reference, Request, Status, Zone, names::Names};
use crate::tencent::{Class, Failure, Page};
use serde_json::Value;
use std::collections::BTreeSet;

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "DNSPod receipt is missing, inconsistent or outside the finite observation model; preserve unknown results",
    )
}

struct Record<'a>(&'a Value);

impl Record<'_> {
    fn text(&self, key: &str) -> Result<String, Failure> {
        let value = self.0[key].as_str().ok_or_else(response)?;
        Names::text(value, 4096).map_err(|_| response())?;
        Ok(value.into())
    }

    fn number(&self, key: &str) -> Result<u64, Failure> {
        self.0[key].as_u64().ok_or_else(response)
    }

    fn positive(&self, key: &str) -> Result<u64, Failure> {
        let value = self.number(key)?;
        if value == 0 {
            return Err(response());
        }
        Ok(value)
    }

    fn optional(&self, key: &str) -> Result<Option<u64>, Failure> {
        self.0
            .get(key)
            .filter(|value| !value.is_null())
            .map(|value| value.as_u64().ok_or_else(response))
            .transpose()
    }

    fn entry(&self, reference: Reference, exact: bool) -> Result<Entry, Failure> {
        let columns = if exact {
            Columns {
                name: "SubDomain",
                kind: "RecordType",
                line: "RecordLine",
                route: "RecordLineId",
            }
        } else {
            Columns {
                name: "Name",
                kind: "Type",
                line: "Line",
                route: "LineId",
            }
        };
        let status = if exact {
            match self.number("Enabled")? {
                0 => Status::Disabled,
                1 => Status::Enabled,
                _ => return Err(response()),
            }
        } else {
            match self.text("Status")?.as_str() {
                "ENABLE" => Status::Enabled,
                "DISABLE" => Status::Disabled,
                _ => return Err(response()),
            }
        };
        let default = self
            .0
            .get("DefaultNS")
            .filter(|value| !value.is_null())
            .map(|value| value.as_bool().ok_or_else(response))
            .transpose()?;
        let name = self.text(columns.name)?;
        Names::host(&name).map_err(|_| response())?;
        let kind = self.text(columns.kind)?;
        Names::text(&kind, 32).map_err(|_| response())?;
        Ok(Entry {
            reference,
            name,
            kind,
            value: self.text("Value")?,
            line: self.text(columns.line)?,
            route: self.text(columns.route)?,
            ttl: self.positive("TTL")?,
            status,
            weight: self.optional("Weight")?,
            mx: self.optional("MX")?,
            default,
        })
    }
}

impl Request {
    pub(super) fn decode(&self, value: &Value) -> Result<Data, Failure> {
        match &self.operation {
            Operation::Domain => self.domain(&value["DomainInfo"]),
            Operation::Get { id } => {
                let record = Record(&value["RecordInfo"]);
                let reference = Reference {
                    domain: Some(record.positive("DomainId")?),
                    record: record.positive("Id")?,
                };
                if reference.record != *id
                    || self
                        .zone
                        .id()
                        .is_some_and(|id| reference.domain != Some(id))
                {
                    return Err(response());
                }
                Ok(Data::Record(record.entry(reference, true)?))
            }
            Operation::List { offset, limit } => self.page(value, (*offset, *limit)),
            Operation::Create(_) | Operation::Modify { .. } => {
                let id = Record(value).positive("RecordId")?;
                if let Operation::Modify { id: expected, .. } = &self.operation
                    && *expected != id
                {
                    return Err(response());
                }
                Ok(Data::Acknowledged(Reference {
                    domain: self.zone.id(),
                    record: id,
                }))
            }
            Operation::Delete { id } => Ok(Data::Acknowledged(Reference {
                domain: self.zone.id(),
                record: *id,
            })),
        }
    }

    fn domain(&self, value: &Value) -> Result<Data, Failure> {
        let record = Record(value);
        let id = record.positive("DomainId")?;
        let name = record.text("Domain")?;
        Names::domain(&name).map_err(|_| response())?;
        let matches = match &self.zone {
            Zone::Name(expected) => expected == &name,
            Zone::Id(expected) => *expected == id,
        };
        if !matches {
            return Err(response());
        }
        let account = value
            .get("Uin")
            .filter(|value| !value.is_null())
            .map(|value| {
                let account = value.as_str().ok_or_else(response)?;
                if account.is_empty() {
                    return Ok(None);
                }
                if account.starts_with('0')
                    || !account.bytes().all(|byte| byte.is_ascii_digit())
                    || account.parse::<u64>().ok().is_none_or(|value| value == 0)
                {
                    return Err(response());
                }
                Ok(Some(account.to_owned()))
            })
            .transpose()?
            .flatten();
        Ok(Data::Domain(Domain {
            account,
            id,
            name,
            status: record.text("Status")?,
            ttl: record.optional("TTL")?,
        }))
    }

    fn page(&self, value: &Value, window: (u32, u32)) -> Result<Data, Failure> {
        let (offset, limit) = window;
        let values = value["RecordList"].as_array().ok_or_else(response)?;
        let count = u32::try_from(values.len()).map_err(|_| response())?;
        let info = Record(&value["RecordCountInfo"]);
        let total = info.number("TotalCount")?;
        let listed = info.number("ListCount")?;
        let capacity = count <= limit;
        let progress = count > 0 || total <= u64::from(offset);
        if listed != u64::from(count) || !capacity || !progress {
            return Err(response());
        }
        let end = offset.checked_add(count).ok_or_else(response)?;
        if count > 0 && u64::from(end) > total {
            return Err(response());
        }
        let mut items = Vec::new();
        let mut ids = BTreeSet::new();
        for value in values {
            let record = Record(value);
            let id = record.positive("RecordId")?;
            if !ids.insert(id) {
                return Err(response());
            }
            items.push(record.entry(
                Reference {
                    domain: self.zone.id(),
                    record: id,
                },
                false,
            )?);
        }
        Ok(Data::Records(Page {
            items,
            total,
            offset,
            limit,
            next: (count > 0 && u64::from(end) < total).then_some(end),
        }))
    }
}

struct Columns {
    name: &'static str,
    kind: &'static str,
    line: &'static str,
    route: &'static str,
}
