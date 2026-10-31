mod recovery;
mod session;

use super::{
    Class, Error, Identity, Message, Operation, Outcome, Receipt, Status, auth,
    fault::response,
    model::{State, identifier, now},
    store::Store,
    wire,
};
use crate::http::wire as http;
use std::path::Path;
use zeroize::Zeroizing;

pub struct Client {
    store: Store,
    state: State,
    agent: http::Client,
    actor: Option<String>,
}

impl Client {
    pub fn open(path: &Path, app: &str) -> Result<Self, Error> {
        identifier(app, "cli_")?;
        let store = Store::open(path)?;
        let state = store.read(app)?;
        Ok(Self {
            store,
            state,
            agent: wire::agent(),
            actor: None,
        })
    }

    pub fn import(path: &Path, app: &str, credentials: &Path) -> Result<Status, Error> {
        let store = Store::open(path)?;
        Ok(store.import(app, credentials)?.status())
    }

    pub fn status(&self) -> Status {
        let mut status = self.state.status();
        if self.actor.is_some() {
            status.actor = super::Actor::User;
            status.expected = self.actor.clone();
            status.expires = status.user.as_ref().and_then(|u| u.expires);
            status.valid = status.user.as_ref().is_some_and(|u| u.valid);
            status.identity = status.user.as_ref().and_then(|u| u.identity.clone());
        }
        status
    }

    pub fn renew(&mut self) -> Result<Status, Error> {
        if self.actor.is_some() {
            return self.refresh();
        }
        auth::renew(&self.agent, &mut self.state)?;
        self.store.save(&self.state)?;
        Ok(self.status())
    }

    pub fn identity(&mut self) -> Result<Identity, Error> {
        if self.actor.is_some() {
            let identity = auth::person(&self.agent, &self.state)?;
            self.person()?.identity = Some(identity.clone());
            self.store.save(&self.state)?;
            return Ok(identity);
        }
        let identity = auth::identity(&self.agent, &self.state)?;
        self.state.identity = Some(identity.clone());
        self.store.save(&self.state)?;
        Ok(identity)
    }

    pub fn user(path: &Path, app: &str, expected: &str) -> Result<Self, Error> {
        identifier(expected, "ou_")?;
        let mut client = Self::open(path, app)?;
        if let Some(user) = &client.state.user {
            user.check(expected)?;
        }
        client.actor = Some(expected.into());
        Ok(client)
    }

    fn binding(&self) -> Result<Identity, Error> {
        match &self.actor {
            Some(_) => self
                .state
                .user
                .as_ref()
                .ok_or_else(super::fault::input)?
                .bound(&self.state.app),
            None => self.state.bound(),
        }
    }

    fn token(&self) -> Result<&str, Error> {
        match &self.actor {
            Some(_) => self
                .state
                .user
                .as_ref()
                .ok_or_else(super::fault::input)?
                .token(),
            None => self.state.token(),
        }
    }

    pub fn messages(&self, page: &super::Page) -> Result<Receipt<super::Messages>, Error> {
        let call = page.call()?;
        let identity = self.binding()?;
        let bearer = Zeroizing::new(format!("Bearer {}", self.token()?));
        let url = format!("https://open.feishu.cn/open-apis/{}", call.path);
        let record = wire::invoke(
            &self.agent,
            http::Request {
                method: "GET",
                url: &url,
                headers: &[("Authorization", &bearer)],
                body: None,
            },
        )?;
        let data = page.decode(&record.body.0).map_err(|_| {
            let mut error = response();
            error.request = record.request.clone();
            error
        })?;
        Ok(Receipt {
            identity,
            action: call.action.into(),
            observed: now(),
            request: record.request,
            outcome: Outcome::Observed,
            data,
        })
    }

    pub fn message(&self, operation: &Operation) -> Result<Receipt<Message>, Error> {
        let call = operation.call()?;
        let identity = self.binding()?;
        let bearer = Zeroizing::new(format!("Bearer {}", self.token()?));
        let body = call
            .body
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| {
                Error::new(
                    Class::Input,
                    "Feishu message encoding refused; nothing sent",
                )
            })?
            .map(Zeroizing::new);
        let url = format!("https://open.feishu.cn/open-apis/{}", call.path);
        let record = wire::invoke(
            &self.agent,
            http::Request {
                method: call.method,
                url: &url,
                headers: &[
                    ("Authorization", &bearer),
                    ("Content-Type", "application/json"),
                ],
                body: body.as_deref().map(Vec::as_slice),
            },
        )?;
        let data = operation.decode(&record.body.0).map_err(|_| {
            let mut error = response();
            error.request = record.request.clone();
            error
        })?;
        if call.method == "POST" && !data.sender.matches(&identity) {
            let mut error = response();
            error.request = record.request;
            return Err(error);
        }
        Ok(Receipt {
            identity,
            action: call.action.into(),
            observed: now(),
            request: record.request,
            outcome: if call.method == "GET" {
                Outcome::Observed
            } else {
                Outcome::Accepted
            },
            data,
        })
    }
}
