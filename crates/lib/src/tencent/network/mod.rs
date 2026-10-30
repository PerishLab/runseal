mod names;
mod policy;
mod query;
mod record;
mod request;
mod survey;
mod types;

pub use policy::{Action, Protocol, Rule, Rules};
pub use query::{Group, Id, Operation, Page, Request, Subnet, Vpc};
pub use survey::{Count, Survey};
pub use types::{Data, Entry, Error, Outcome, Receipt};

use super::{Class, Failure};

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "Tencent network input refused; use explicit region, native IDs, bounded names/CIDRs and nonzero whole-policy version",
    )
}

impl super::Client {
    pub fn network(&self, request: &Request) -> Result<Receipt, Error> {
        let unsent = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let call = request.call().map_err(unsent)?;
        let identity = self.binding().map_err(unsent)?;
        let client = self.api().map_err(unsent)?;
        let uncertain = |fault| Error {
            fault,
            outcome: Outcome::Unknown,
        };
        let body = client
            .invoke(call.request(&request.region))
            .map_err(|error| {
                let fault = Failure::capture(error, Class::Response);
                Error::remote(fault)
            })?;
        let id = body.0["RequestId"]
            .as_str()
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 128
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
            .ok_or_else(|| {
                uncertain(Failure::new(
                    Class::Response,
                    "network receipt lacks a safe RequestId",
                ))
            })?;
        let data = call.decode(&body.0).map_err(uncertain)?;
        data.regional(&request.region).map_err(uncertain)?;
        Ok(Receipt {
            identity,
            service: "vpc".into(),
            version: "2017-03-12".into(),
            action: call.action.into(),
            region: request.region.clone(),
            observed: super::model::now(),
            request: id.into(),
            outcome: if call.mutation() {
                Outcome::Accepted
            } else {
                Outcome::Observed
            },
            data,
        })
    }
}

#[cfg(test)]
#[path = "checks/native.rs"]
mod checks;
#[cfg(test)]
#[path = "checks/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "checks/wire.rs"]
mod wire;

#[cfg(test)]
#[path = "checks/survey.rs"]
mod surveys;

#[cfg(test)]
#[path = "checks/transport.rs"]
mod transport;
