use super::{Receipt, record};
use crate::tencent::{Class, Error, Failure, Outcome, api};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Entities { policy: u64, page: u32 },
    Delete { policy: u64 },
}

impl Request {
    pub fn validate(&self) -> Result<(), Failure> {
        if !(1..=i64::MAX as u64).contains(&self.policy())
            || matches!(self, Self::Entities { page: 0, .. })
        {
            return Err(Failure::new(
                Class::Input,
                "CAM policy operation requires one positive policy ID and a positive page",
            ));
        }
        Ok(())
    }

    pub(super) fn policy(&self) -> u64 {
        match self {
            Self::Entities { policy, .. } | Self::Delete { policy } => *policy,
        }
    }

    pub(super) fn action(&self) -> &'static str {
        match self {
            Self::Entities { .. } => "ListEntitiesForPolicy",
            Self::Delete { .. } => "DeletePolicy",
        }
    }

    pub(super) fn params(&self) -> Result<Value, Failure> {
        self.validate()?;
        Ok(match self {
            Self::Entities { policy, page } => {
                json!({"PolicyId":policy,"Page":page,"Rp":100,"EntityFilter":"All"})
            }
            Self::Delete { policy } => json!({"PolicyId":[policy]}),
        })
    }
}

impl crate::tencent::Client {
    pub fn cam(&self, request: &Request) -> Result<Receipt, Error> {
        let unsent = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let params = request.params().map_err(unsent)?;
        let identity = self.binding().map_err(unsent)?;
        let client = self.api().map_err(unsent)?;
        let body = client
            .invoke(api::Request {
                service: "cam",
                action: request.action(),
                region: None,
                params: &params,
            })
            .map_err(|error| Error::remote(Failure::capture(error, Class::Response)))?;
        record::receipt(request, identity, &body.0).map_err(|fault| Error {
            fault,
            outcome: Outcome::Unknown,
        })
    }
}
