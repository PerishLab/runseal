mod instance;
mod key;
mod names;
mod record;
mod request;
mod types;

pub use super::Window;
use super::{Class, Error, Failure, Lookup, Outcome};
pub use instance::{Addresses, Instance, Login, Progress};
pub use key::{Association, Key};
pub use request::{Operation, Request};
pub use types::{Data, Extension, Label, Opaque, Page, Platform, Receipt, Schema, Shape, Stage};

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "Lighthouse read requires an explicit region, exact native ID or bounded key page",
    )
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "Lighthouse observation malformed, conflicting or outside the finite read model",
    )
}

impl super::Client {
    pub fn lighthouse(&self, request: &Request) -> Result<Receipt, Error> {
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
        request.receipt(identity, &body.0).map_err(|fault| Error {
            fault,
            outcome: Outcome::Unknown,
        })
    }
}

#[cfg(test)]
mod tests;
