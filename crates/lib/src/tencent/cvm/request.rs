use super::{Data, Failure, Import, Spec, input, names::Names};
use crate::tencent::api;
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Request {
    pub region: String,
    pub operation: Operation,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Quote(Spec),
    Quota {
        zone: Option<String>,
    },
    Run {
        spec: Spec,
        token: String,
        dry: bool,
    },
    Get {
        id: String,
    },
    Status {
        id: String,
    },
    Rename {
        id: String,
        name: String,
    },
    Terminate {
        id: String,
    },
    Import(Import),
    Key {
        id: String,
    },
    Delete {
        id: String,
    },
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
            Operation::Quote(spec) => ("InquiryPriceRunInstances", spec.body(&self.region)?),
            Operation::Quota { zone } => ("DescribeAccountQuota", self.quota(zone.as_deref())?),
            Operation::Run { spec, token, dry } => {
                Names::token(token)?;
                let mut body = spec.body(&self.region)?;
                body["ClientToken"] = json!(token);
                body["DryRun"] = json!(dry);
                ("RunInstances", body)
            }
            Operation::Get { id } | Operation::Status { id } => {
                Names::id(id, "ins-")?;
                let action = if matches!(self.operation, Operation::Get { .. }) {
                    "DescribeInstances"
                } else {
                    "DescribeInstancesStatus"
                };
                (action, json!({"InstanceIds":[id],"Offset":0,"Limit":1}))
            }
            Operation::Rename { id, name } => {
                Names::id(id, "ins-")?;
                Names::text(name, 60)?;
                if name.contains(['{', '}']) {
                    return Err(input());
                }
                (
                    "ModifyInstancesAttribute",
                    json!({"InstanceIds":[id],"InstanceName":name}),
                )
            }
            Operation::Terminate { id } => {
                Names::id(id, "ins-")?;
                (
                    "TerminateInstances",
                    json!({"InstanceIds":[id],"ReleaseAddress":false,"ReleasePrepaidDataDisks":false}),
                )
            }
            Operation::Import(key) => ("ImportKeyPair", key.body()?),
            Operation::Key { id } => {
                Names::id(id, "skey-")?;
                (
                    "DescribeKeyPairs",
                    json!({"KeyIds":[id],"Offset":0,"Limit":1}),
                )
            }
            Operation::Delete { id } => {
                Names::id(id, "skey-")?;
                ("DeleteKeyPairs", json!({"KeyIds":[id]}))
            }
        };
        Ok(Call { action, body })
    }

    fn quota(&self, zone: Option<&str>) -> Result<Value, Failure> {
        let Some(zone) = zone else {
            return Ok(json!({}));
        };
        Names::zone(zone, &self.region)?;
        Ok(json!({"Filters":[{"Name":"zone","Values":[zone]}]}))
    }

    pub(super) fn mutation(&self) -> bool {
        matches!(
            self.operation,
            Operation::Run { dry: false, .. }
                | Operation::Rename { .. }
                | Operation::Terminate { .. }
                | Operation::Import(_)
                | Operation::Delete { .. }
        )
    }

    pub(super) fn decode(&self, value: &Value) -> Result<Data, Failure> {
        super::record::decode(self, value)
    }
}

impl Call {
    pub(super) fn request<'a>(&'a self, region: &'a str) -> api::Request<'a> {
        api::Request {
            service: "cvm",
            action: self.action,
            region: Some(region),
            params: &self.body,
        }
    }
}
