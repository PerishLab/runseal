mod ceremony;
mod exchange;
mod result;

use super::{Class, Error, Failure, Outcome, secret::Store};
use serde_json::{Value, json};
use std::path::Path;

pub struct Auth {
    store: Store,
}

impl Auth {
    pub fn new(path: &Path) -> Result<Self, Failure> {
        Store::open(path)
            .map(|store| Self { store })
            .map_err(|error| Failure::capture(error, Class::Custody))
    }

    pub fn begin(&self) -> Result<Value, Error> {
        ceremony::begin(&self.store).map_err(local)
    }

    pub fn complete(&self, response: &Path) -> Result<Value, Error> {
        ceremony::complete(&self.store, response).map_err(local)
    }

    pub fn refresh(&self) -> Result<Value, Error> {
        exchange::refresh(&self.store).map_err(local)
    }

    pub fn renew(&self) -> Result<Value, Error> {
        exchange::renew(&self.store).map_err(local)
    }

    pub fn recover(&self) -> Result<Value, Error> {
        let mut state = self.store.read().map_err(local)?;
        result::settle(&self.store, &mut state).map_err(local)
    }

    pub fn status(&self) -> Result<Value, Error> {
        self.store
            .read()
            .map(|state| state.receipt())
            .map_err(local)
    }

    pub fn logout(&self) -> Result<Value, Error> {
        self.store.read().map_err(local)?;
        save(&self.store, &super::model::State::fresh(), false).map_err(local)?;
        Ok(json!({"localLogout":true,"remoteRevocation":false}))
    }
}

fn local(error: anyhow::Error) -> Error {
    error
        .downcast_ref::<Error>()
        .cloned()
        .unwrap_or_else(|| Error {
            fault: Failure::capture(error, Class::Input),
            outcome: Outcome::Unsent,
        })
}

fn unknown(message: &'static str) -> Error {
    Error {
        fault: Failure::new(Class::Response, message),
        outcome: Outcome::Unknown,
    }
}

fn save(store: &Store, state: &super::model::State, dispatched: bool) -> anyhow::Result<()> {
    store.save(state).map_err(|error| {
        Error {
            fault: Failure::capture(error, Class::Custody),
            outcome: if dispatched {
                Outcome::Unknown
            } else {
                Outcome::Unsent
            },
        }
        .into()
    })
}

fn available(state: &super::model::State) -> anyhow::Result<()> {
    if state.pending.is_some() || state.retained.is_some() {
        return Err(unknown("Alibaba token operation remains uncertain; inspect status and recover the retained return or explicitly discard local custody; never replay").into());
    }
    Ok(())
}
