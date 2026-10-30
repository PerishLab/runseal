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
        })
    }

    pub fn import(path: &Path, app: &str, credentials: &Path) -> Result<Status, Error> {
        let store = Store::open(path)?;
        Ok(store.import(app, credentials)?.status())
    }

    pub fn status(&self) -> Status {
        self.state.status()
    }

    pub fn renew(&mut self) -> Result<Status, Error> {
        auth::renew(&self.agent, &mut self.state)?;
        self.store.save(&self.state)?;
        Ok(self.status())
    }

    pub fn identity(&mut self) -> Result<Identity, Error> {
        let identity = auth::identity(&self.agent, &self.state)?;
        self.state.identity = Some(identity.clone());
        self.store.save(&self.state)?;
        Ok(identity)
    }

    pub fn message(&self, operation: &Operation) -> Result<Receipt<Message>, Error> {
        let call = operation.call()?;
        let identity = self.state.bound()?;
        let bearer = Zeroizing::new(format!("Bearer {}", self.state.token()?));
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
