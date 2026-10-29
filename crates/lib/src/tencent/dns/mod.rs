mod names;
mod query;
mod record;
mod types;

use super::{Class, Error, Failure, Outcome};
pub use query::{Operation, Request};
pub use types::{Data, Domain, Entry, Fields, Kind, Receipt, Reference, Remote, Status, Zone};

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "DNSPod input refused; use a canonical zone name or positive DomainId, bounded public fields and explicit record IDs",
    )
}

impl super::Client {
    pub fn dns(&self, request: &Request) -> Result<Receipt, Error> {
        let unsent = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let call = request.call().map_err(unsent)?;
        let identity = self.binding().map_err(unsent)?;
        let client = self.api().map_err(unsent)?;
        let body = client
            .invoke(call.request())
            .map_err(|error| Error::remote(Failure::capture(error, Class::Response)))?;
        let uncertain = |fault| Error {
            fault,
            outcome: Outcome::Unknown,
        };
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
                    "DNSPod receipt lacks a safe RequestId",
                ))
            })?;
        let data = request.decode(&body.0).map_err(|mut fault| {
            fault.request = Some(id.into());
            uncertain(fault)
        })?;
        Ok(Receipt {
            identity,
            service: "dnspod".into(),
            version: "2021-03-23".into(),
            action: call.action.into(),
            region: None,
            zone: request.zone.clone(),
            request: id.into(),
            observed: super::model::now(),
            outcome: if request.mutation() {
                Outcome::Accepted
            } else {
                Outcome::Observed
            },
            data,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod checks;
#[cfg(test)]
mod wire;
