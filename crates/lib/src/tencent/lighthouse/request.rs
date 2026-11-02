use super::{Data, Failure, Lookup, Page, Receipt, Window, names::Names, record::Record, response};
use crate::tencent::{Identity, Outcome, api};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Request {
    pub region: String,
    pub operation: Operation,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Instance { id: String },
    Key { id: String },
    Keys(Window),
}

pub(super) struct Call {
    pub action: &'static str,
    pub body: Value,
}

impl Request {
    pub fn validate(&self) -> Result<(), Failure> {
        self.call().map(|_| ())
    }

    pub(super) fn call(&self) -> Result<Call, Failure> {
        Names::region(&self.region)?;
        let (action, body) = match &self.operation {
            Operation::Instance { id } => {
                Names::id(id, "lhins-")?;
                (
                    "DescribeInstances",
                    json!({"InstanceIds":[id],"Offset":0,"Limit":1}),
                )
            }
            Operation::Key { id } => {
                Names::id(id, "lhkp-")?;
                (
                    "DescribeKeyPairs",
                    json!({"KeyIds":[id],"Offset":0,"Limit":1}),
                )
            }
            Operation::Keys(window) => {
                if !(1..=100).contains(&window.limit) {
                    return Err(super::input());
                }
                (
                    "DescribeKeyPairs",
                    json!({"Offset":window.offset,"Limit":window.limit}),
                )
            }
        };
        Ok(Call { action, body })
    }

    pub(super) fn decode(&self, body: &Value) -> Result<Data, Failure> {
        let record = Record(body);
        let total = record.number("TotalCount")?;
        match &self.operation {
            Operation::Instance { id } => {
                let rows = record.array("InstanceSet", 1)?.ok_or_else(response)?;
                if total != rows.len() as u64 {
                    return Err(response());
                }
                let value = rows
                    .first()
                    .map(|row| Record(row).instance(&self.region))
                    .transpose()?;
                if value.as_ref().is_some_and(|value| &value.id != id) {
                    return Err(response());
                }
                let complete = record.known(&["InstanceSet", "TotalCount", "RequestId"])
                    && value.as_ref().is_none_or(|value| value.complete);
                Ok(Data::Instance(Lookup {
                    id: id.clone(),
                    value: value.map(Box::new),
                    total,
                    complete,
                }))
            }
            Operation::Key { id } => {
                let page = self.keys(
                    body,
                    Window {
                        offset: 0,
                        limit: 1,
                    },
                )?;
                if total != page.entries.len() as u64 {
                    return Err(response());
                }
                let complete = page.complete;
                let value = page.entries.into_iter().next();
                if value.as_ref().is_some_and(|value| &value.id != id) {
                    return Err(response());
                }
                Ok(Data::Key(Lookup {
                    id: id.clone(),
                    value,
                    total,
                    complete,
                }))
            }
            Operation::Keys(window) => Ok(Data::Keys(self.keys(body, *window)?)),
        }
    }

    fn keys(&self, body: &Value, window: Window) -> Result<Page, Failure> {
        let record = Record(body);
        let total = record.number("TotalCount")?;
        let rows = record
            .array("KeyPairSet", window.limit as usize)?
            .ok_or_else(response)?;
        let end = u64::from(window.offset) + rows.len() as u64;
        if end > total || (end < total && rows.is_empty()) {
            return Err(response());
        }
        let next = if end < total {
            Some(u32::try_from(end).map_err(|_| response())?)
        } else {
            None
        };
        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        for row in rows {
            let value = Record(row).key()?;
            if !seen.insert(value.id.clone()) {
                return Err(response());
            }
            entries.push(value);
        }
        let complete = record.known(&["KeyPairSet", "TotalCount", "RequestId"])
            && entries.iter().all(|value| value.complete);
        Ok(Page {
            entries,
            total,
            offset: window.offset,
            limit: window.limit,
            next,
            complete,
        })
    }

    pub(super) fn receipt(&self, identity: Identity, body: &Value) -> Result<Receipt, Failure> {
        let call = self.call()?;
        let request = body["RequestId"]
            .as_str()
            .filter(|value| Names::request(value))
            .ok_or_else(response)?;
        let data = self.decode(body).map_err(|mut fault| {
            fault.request = Some(request.into());
            fault
        })?;
        Ok(Receipt {
            identity,
            service: "lighthouse".into(),
            version: "2020-03-24".into(),
            action: call.action.into(),
            region: self.region.clone(),
            request: request.into(),
            observed: crate::tencent::model::now(),
            outcome: Outcome::Observed,
            data,
        })
    }
}

impl Call {
    pub(super) fn request<'a>(&'a self, region: &'a str) -> api::Request<'a> {
        api::Request {
            service: "lighthouse",
            action: self.action,
            region: Some(region),
            params: &self.body,
        }
    }
}
