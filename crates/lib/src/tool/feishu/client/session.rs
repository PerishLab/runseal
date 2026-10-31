use super::*;

impl Client {
    pub fn begin(&mut self, scopes: &[String]) -> Result<super::super::Challenge, Error> {
        let scopes = auth::scopes(scopes)?;
        if !scopes.iter().any(|scope| scope == "offline_access") {
            return Err(Error::new(
                Class::Input,
                "Feishu user authorization requires explicit offline_access scope for durable refresh",
            ));
        }
        let expected = self
            .actor
            .as_deref()
            .ok_or_else(super::super::fault::input)?;
        if self.state.user.is_some() {
            return Err(Error::new(
                Class::Binding,
                "Feishu user session already exists; inspect status or explicitly discard local user session before new authorization",
            ));
        }
        let mut user = auth::User::new(expected);
        user.uncertain = true;
        self.state.version = 2;
        self.state.user = Some(user);
        self.store.save(&self.state)?;
        let device = auth::begin(&self.agent, &self.state, &scopes)?;
        let challenge = device.challenge.clone();
        let user = self.person()?;
        user.device = Some(device);
        user.uncertain = false;
        self.store.save(&self.state)?;
        Ok(challenge)
    }

    pub fn exchange(&mut self) -> Result<Status, Error> {
        let user = self.person()?;
        if user.uncertain {
            return Err(Error::new(
                Class::Binding,
                "Feishu exchange uncertain; inspect status, then discard local session and reauthorize if unrecoverable",
            ));
        }
        let device = user
            .device
            .as_ref()
            .ok_or_else(super::super::fault::input)?;
        if now() >= device.challenge.expires {
            return Err(Error::new(
                Class::Expired,
                "Feishu device authorization expired; discard local session and begin again",
            ));
        }
        if now() < device.next {
            return Err(Error::new(
                Class::Limit,
                "Feishu device exchange interval not reached; inspect status before explicit next exchange",
            ));
        }
        user.uncertain = true;
        self.store.save(&self.state)?;
        match auth::exchange(&self.agent, &self.state) {
            Ok(grant) => {
                let user = self.person()?;
                user.grant = Some(grant);
                user.device = None;
                user.identity = None;
                user.uncertain = false;
            }
            Err(error) => {
                self.pending(&error)?;
                return Err(error);
            }
        }
        self.store.save(&self.state)?;
        Ok(self.status())
    }

    fn pending(&mut self, error: &Error) -> Result<(), Error> {
        if let Some(poll) = error.poll {
            let user = self.person()?;
            let device = user
                .device
                .as_mut()
                .ok_or_else(super::super::fault::input)?;
            let increase = if poll == super::super::fault::Poll::Slow {
                5
            } else {
                0
            };
            device.challenge.interval += increase;
            device.next = now() + device.challenge.interval;
            user.uncertain = false;
            self.store.save(&self.state)?;
        }
        Ok(())
    }

    pub fn logout(&mut self) -> Result<Status, Error> {
        if self.actor.is_none() {
            return Err(super::super::fault::input());
        }
        self.state.user = None;
        self.store.save(&self.state)?;
        Ok(self.status())
    }

    pub(super) fn refresh(&mut self) -> Result<Status, Error> {
        let user = self.person()?;
        if user.uncertain {
            return Err(Error::new(
                Class::Binding,
                "Feishu refresh uncertain; do not replay; discard local user session and reauthorize if unrecoverable",
            ));
        }
        let grant = user.grant.as_ref().ok_or_else(super::super::fault::input)?;
        if grant.expires <= now() + 30 {
            return Err(Error::new(
                Class::Expired,
                "Feishu refresh credential expired; explicitly reauthorize",
            ));
        }
        user.uncertain = true;
        self.store.save(&self.state)?;
        let grant = auth::refresh(&self.agent, &self.state)?;
        let user = self.person()?;
        user.grant = Some(grant);
        user.identity = None;
        user.uncertain = false;
        self.store.save(&self.state)?;
        Ok(self.status())
    }

    pub(super) fn person(&mut self) -> Result<&mut auth::User, Error> {
        let expected = self
            .actor
            .as_deref()
            .ok_or_else(super::super::fault::input)?;
        let user = self
            .state
            .user
            .as_mut()
            .ok_or_else(super::super::fault::input)?;
        user.check(expected)?;
        Ok(user)
    }
}
