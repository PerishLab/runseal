use super::{Fields, Zone, input, names::Names};
use crate::tencent::{Failure, api};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Request {
    pub zone: Zone,
    pub operation: Operation,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Domain,
    Get { id: u64 },
    List { offset: u32, limit: u32 },
    Create(Fields),
    Modify { id: u64, fields: Fields },
    Delete { id: u64 },
}

pub(super) struct Call {
    pub action: &'static str,
    pub body: Value,
}

impl Zone {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        match self {
            Self::Name(name) => {
                Names::domain(name)?;
                Ok(json!({"Domain":name}))
            }
            Self::Id(id) => {
                Names::id(*id)?;
                Ok(json!({"DomainId":id}))
            }
        }
    }

    pub(super) fn id(&self) -> Option<u64> {
        match self {
            Self::Id(id) => Some(*id),
            _ => None,
        }
    }
}

impl Request {
    pub(super) fn call(&self) -> Result<Call, Failure> {
        let mut body = self.zone.body()?;
        let action = match &self.operation {
            Operation::Domain => "DescribeDomain",
            Operation::Get { id } => {
                Names::id(*id)?;
                body["RecordId"] = json!(id);
                "DescribeRecord"
            }
            Operation::List { offset, limit } => {
                if !(1..=3000).contains(limit) {
                    return Err(input());
                }
                body["Offset"] = json!(offset);
                body["Limit"] = json!(limit);
                body["ErrorOnEmpty"] = json!("no");
                "DescribeRecordList"
            }
            Operation::Create(fields) => {
                body.as_object_mut()
                    .expect("zone object")
                    .extend(fields.body()?.as_object().expect("fields object").clone());
                "CreateRecord"
            }
            Operation::Modify { id, fields } => {
                Names::id(*id)?;
                body["RecordId"] = json!(id);
                body.as_object_mut()
                    .expect("zone object")
                    .extend(fields.body()?.as_object().expect("fields object").clone());
                "ModifyRecord"
            }
            Operation::Delete { id } => {
                Names::id(*id)?;
                body["RecordId"] = json!(id);
                "DeleteRecord"
            }
        };
        Ok(Call { action, body })
    }

    pub(super) fn mutation(&self) -> bool {
        matches!(
            self.operation,
            Operation::Create(_) | Operation::Modify { .. } | Operation::Delete { .. }
        )
    }
}

impl Call {
    pub(super) fn request(&self) -> api::Request<'_> {
        api::Request {
            service: "dnspod",
            action: self.action,
            region: None,
            params: &self.body,
        }
    }
}

impl Fields {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        Names::host(&self.name)?;
        Names::text(&self.line, 64)?;
        Names::text(&self.value, 4096)?;
        if !(1..=604800).contains(&self.ttl) {
            return Err(input());
        }
        Names::value(self.kind, &self.value)?;
        Ok(
            json!({"SubDomain":self.name,"RecordType":self.kind.name(),"Value":self.value,"RecordLine":self.line,"TTL":self.ttl}),
        )
    }
}
