mod instance;
mod key;
mod names;
mod price;
mod record;
mod request;
mod spec;
mod types;

use super::{Class, Error, Failure, Outcome};
pub use instance::{Attachment, Facts, Instance, Progress, Stage, Status};
pub use key::{Import, Key};
pub use price::{Price, Quota, Rate, Tier};
pub use request::{Operation, Request};
pub use spec::{Connection, Disk, Hardware, Placement, Spec, Storage};
pub use types::{Data, Receipt};

pub(super) fn input() -> Failure {
    Failure::new(
        Class::Input,
        "CVM input refused; use one hourly instance, explicit scope, bounded public fields and an imported RSA public key",
    )
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "CVM receipt incomplete, inconsistent or outside the finite model; preserve unknown results",
    )
}

impl super::Client {
    pub fn cvm(&self, request: &Request) -> Result<Receipt, Error> {
        let unsent = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let call = request.call().map_err(unsent)?;
        let identity = self.binding().map_err(unsent)?;
        let client = self.api().map_err(unsent)?;
        let body = client
            .invoke(call.request(&request.region))
            .map_err(|error| Error::remote(Failure::capture(error, Class::Response)))?;
        let uncertain = |fault| Error {
            fault,
            outcome: Outcome::Unknown,
        };
        let id = body.0["RequestId"]
            .as_str()
            .filter(|id| names::Names::request(id).is_ok())
            .ok_or_else(|| uncertain(response()))?;
        let data = request.decode(&body.0).map_err(|mut fault| {
            fault.request = Some(id.into());
            uncertain(fault)
        })?;
        Ok(Receipt {
            identity,
            service: "cvm".into(),
            version: "2017-03-12".into(),
            action: call.action.into(),
            region: request.region.clone(),
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
