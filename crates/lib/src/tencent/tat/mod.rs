mod names;
mod record;
mod request;
mod types;

use super::{Class, Error, Failure, Outcome};
pub use request::{Operation, Request};
pub use types::{Agent, Availability, Data, Environment, Feature, Receipt, Region, Status};

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "TAT requires an explicit region and one exact Lighthouse instance for agent reads",
    )
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "TAT observation incomplete or inconsistent; no eligibility or absence is inferred",
    )
}

impl super::Client {
    pub fn tat(&self, request: &Request) -> Result<Receipt, Error> {
        let unsent = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let call = request.call().map_err(unsent)?;
        let identity = self.binding().map_err(unsent)?;
        let body = self
            .api()
            .map_err(unsent)?
            .invoke(call.request(&request.region))
            .map_err(|error| Error::remote(Failure::capture(error, Class::Response)))?;
        request.receipt(identity, &body.0).map_err(|fault| Error {
            fault,
            outcome: Outcome::Unknown,
        })
    }
}

#[cfg(test)]
mod tests;
