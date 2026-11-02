use super::{Data, Failure, Receipt, names::Names};
use crate::tencent::{Identity, Outcome, api};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Request {
    pub region: String,
    pub operation: Operation,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Regions,
    Lighthouse { id: String },
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
            Operation::Regions => ("DescribeRegions", json!({})),
            Operation::Lighthouse { id } => {
                Names::instance(id)?;
                (
                    "DescribeAutomationAgentStatus",
                    json!({"InstanceIds":[id],"Offset":0,"Limit":1}),
                )
            }
        };
        Ok(Call { action, body })
    }

    pub(super) fn decode(&self, body: &Value) -> Result<Data, Failure> {
        super::record::decode(self, body)
    }

    pub(super) fn receipt(&self, identity: Identity, body: &Value) -> Result<Receipt, Failure> {
        let call = self.call()?;
        let request = body["RequestId"]
            .as_str()
            .filter(|id| Names::request(id))
            .ok_or_else(super::response)?;
        let data = self.decode(body).map_err(|mut fault| {
            fault.request = Some(request.into());
            fault
        })?;
        Ok(Receipt {
            identity,
            service: "tat".into(),
            version: "2020-10-28".into(),
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
            service: "tat",
            action: self.action,
            region: Some(region),
            params: &self.body,
        }
    }
}
