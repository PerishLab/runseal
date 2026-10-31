use super::super::{
    Class, Error, Identity,
    fault::input,
    model::{Credential, identifier, now, stamp},
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub expected: String,
    pub expires: Option<i64>,
    pub renewable: Option<i64>,
    pub scopes: Vec<String>,
    pub valid: bool,
    pub identity: Option<Identity>,
    pub pending: bool,
    pub challenge: Option<Challenge>,
    pub next: Option<i64>,
    pub uncertain: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Challenge {
    pub verification: String,
    pub code: String,
    pub expires: i64,
    pub interval: i64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Device {
    pub secret: String,
    pub challenge: Challenge,
    pub next: i64,
    pub scopes: Vec<String>,
}

impl Drop for Device {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Grant {
    pub credential: Credential,
    pub refresh: String,
    pub expires: i64,
    pub scopes: Vec<String>,
}

impl Drop for Grant {
    fn drop(&mut self) {
        self.refresh.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct User {
    pub expected: String,
    pub grant: Option<Grant>,
    pub identity: Option<Identity>,
    pub device: Option<Device>,
    pub uncertain: bool,
}

impl User {
    pub fn new(expected: &str) -> Self {
        Self {
            expected: expected.into(),
            grant: None,
            identity: None,
            device: None,
            uncertain: false,
        }
    }

    pub fn token(&self) -> Result<&str, Error> {
        if self.uncertain {
            return Err(uncertain());
        }
        let grant = self.grant.as_ref().ok_or_else(|| Error::new(Class::Authentication,
            "Feishu user token absent; explicitly begin user authorization, complete browser consent, then exchange"))?;
        if grant.credential.expires <= now() + 30 {
            return Err(Error::new(
                Class::Expired,
                "Feishu user token expired; explicitly renew and verify identity",
            ));
        }
        super::super::model::visible(&grant.credential.token, 8192)?;
        Ok(&grant.credential.token)
    }

    pub fn bound(&self, app: &str) -> Result<Identity, Error> {
        let token = self.token()?;
        self.identity.as_ref()
            .filter(|id| id.app == app && id.user.as_deref() == Some(&self.expected))
            .filter(|id| id.bot.is_empty() && id.stamp == stamp(token) && id.observed > 0)
            .cloned().ok_or_else(|| Error::new(Class::Binding,
                "Feishu user identity not verified for expected app/user/token; explicitly run auth identity"))
    }

    pub fn status(&self, app: &str) -> Session {
        Session {
            expected: self.expected.clone(),
            expires: self.grant.as_ref().map(|g| g.credential.expires),
            renewable: self.grant.as_ref().map(|g| g.expires),
            scopes: self
                .grant
                .as_ref()
                .map(|g| g.scopes.clone())
                .unwrap_or_default(),
            valid: self.token().is_ok(),
            identity: self.bound(app).ok(),
            pending: self.device.is_some(),
            challenge: self.device.as_ref().map(|d| d.challenge.clone()),
            next: self.device.as_ref().map(|d| d.next),
            uncertain: self.uncertain,
        }
    }

    pub fn check(&self, expected: &str) -> Result<(), Error> {
        identifier(expected, "ou_")?;
        if self.expected != expected {
            return Err(Error::new(
                Class::Binding,
                "Feishu private user state differs from the expected user",
            ));
        }
        Ok(())
    }
}

pub(super) fn uncertain() -> Error {
    Error::new(
        Class::Binding,
        "Feishu authorization result uncertain; do not replay exchange/refresh; observe local status, then explicitly discard local user session and reauthorize if unrecoverable",
    )
}

pub(crate) fn scopes(values: &[String]) -> Result<Vec<String>, Error> {
    if values.is_empty() || values.len() > 32 {
        return Err(input());
    }
    let mut result = values.to_vec();
    for value in &result {
        if value.len() > 160
            || value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_:.-".contains(&b))
        {
            return Err(input());
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}
