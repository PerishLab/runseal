#[cfg(test)]
mod checks;
mod query;
mod record;
#[cfg(test)]
mod tests;
mod types;

pub use query::{Product, Query, Window};
pub use types::{Bundle, Data, Identity, Image, Instance, Page, Read, Region, Shape};

use super::model::{self, State};
use super::secret::Store;
use super::{Class, Failure, api};
use std::path::Path;

pub struct Client {
    store: Store,
    state: State,
    account: String,
}

impl Client {
    pub fn open(path: &Path, account: &str) -> Result<Self, Failure> {
        if !path.is_absolute()
            || !account.bytes().all(|byte| byte.is_ascii_digit())
            || account.parse::<u64>().ok().is_none_or(|value| value == 0)
        {
            return Err(Failure::new(
                Class::Input,
                "Tencent reader requires an absolute private state path and an explicit owning account",
            ));
        }
        let store = Store::open(path).map_err(|error| Failure::capture(error, Class::Custody))?;
        Self::from(store, account)
    }

    pub(super) fn from(store: Store, account: &str) -> Result<Self, Failure> {
        if !account.bytes().all(|byte| byte.is_ascii_digit())
            || account.parse::<u64>().ok().is_none_or(|value| value == 0)
        {
            return Err(Failure::new(
                Class::Input,
                "Tencent reader requires an explicit owning account",
            ));
        }
        let state = store
            .read()
            .map_err(|error| Failure::capture(error, Class::Custody))?;
        Ok(Self {
            store,
            state,
            account: account.into(),
        })
    }

    pub fn identity(&mut self) -> Result<Identity, Failure> {
        self.credential()?;
        super::identity(&self.store, &mut self.state, Some(&self.account))
            .map_err(|error| Failure::capture(error, Class::Binding))?;
        self.binding()
    }

    pub fn read(&self, query: &Query) -> Result<Read, Failure> {
        let call = query.call()?;
        let identity = self.binding()?;
        let body = api::Client::new(self.credential()?)
            .invoke(call.request())
            .map_err(|error| Failure::capture(error, Class::Response))?;
        let request = model::field(&body.0, "RequestId")
            .map_err(|error| Failure::capture(error, Class::Response))?
            .to_string();
        let data = call.decode(&body.0).map_err(|mut error| {
            error.request = Some(request.clone());
            error
        })?;
        let version = api::version(call.service)
            .map_err(|error| Failure::capture(error, Class::Unsupported))?;
        Ok(Read {
            identity,
            product: call.product,
            service: call.service.into(),
            action: call.action.into(),
            version: version.into(),
            region: call.region,
            observed: model::now(),
            request,
            data,
        })
    }

    fn credential(&self) -> Result<&model::Credential, Failure> {
        let class = if self
            .state
            .credential
            .as_ref()
            .and_then(|value| value.expires)
            .is_some_and(|end| end <= model::now())
        {
            Class::Expired
        } else {
            Class::Authentication
        };
        self.state.credential().map_err(|_| {
            Failure::new(
                class,
                "Tencent credentials unavailable or expired; explicitly renew and verify identity",
            )
        })
    }

    fn binding(&self) -> Result<Identity, Failure> {
        let credential = self.credential()?;
        super::authorize(&self.state, Some(&self.account)).map_err(|_| binding())?;
        let held = self.state.identity.as_ref().ok_or_else(binding)?;
        Ok(Identity {
            account: self.account.clone(),
            principal: model::field(held, "PrincipalId")
                .map_err(|_| binding())?
                .into(),
            kind: model::field(held, "Type").map_err(|_| binding())?.into(),
            stamp: model::stamp(credential),
            observed: held["verifiedAt"].as_u64().ok_or_else(binding)?,
            request: model::field(held, "RequestId")
                .map_err(|_| binding())?
                .into(),
        })
    }
}

fn binding() -> Failure {
    Failure::new(
        Class::Binding,
        "Verify Tencent caller identity against the explicit owning account and current credentials before compute reads",
    )
}
