use super::*;

impl Client {
    pub(super) fn retain(&mut self, returned: auth::Return) -> Result<Status, Error> {
        self.person()?.returned = Some(returned);
        self.state.version = 3;
        self.store.save(&self.state)?;
        self.recover()
    }

    pub fn recover(&mut self) -> Result<Status, Error> {
        let user = self.person()?;
        if !user.uncertain {
            return Err(super::super::fault::input());
        }
        let returned=user.returned.as_ref().ok_or_else(|| Error::new(Class::Binding,
            "No retained Feishu token return; cannot recover locally. Preserve evidence; explicitly discard local session and reauthorize if unrecoverable. Do not replay exchange/refresh"))?;
        let previous = match returned.kind {
            auth::Kind::Exchange => None,
            auth::Kind::Renew => Some(user.grant.as_ref().ok_or_else(super::super::fault::input)?),
        };
        let body = returned.data()?;
        let grant = auth::interpret(&body.0, returned.started, previous).map_err(|mut error| {
            error.request = returned.request.clone();
            error
        })?;
        user.grant = Some(grant);
        user.returned = None;
        user.device = None;
        user.identity = None;
        user.uncertain = false;
        self.store.save(&self.state)?;
        Ok(self.status())
    }
}

#[cfg(all(test, unix))]
#[path = "../tests/recovery.rs"]
mod tests;
