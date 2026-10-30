use super::{Class, Error, Outcome, fault::input};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Identity {
    pub app: String,
    pub bot: String,
    pub observed: i64,
    pub stamp: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub app: String,
    pub expires: Option<i64>,
    pub valid: bool,
    pub identity: Option<Identity>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Receipt<T> {
    pub identity: Identity,
    pub action: String,
    pub observed: i64,
    pub request: Option<String>,
    pub outcome: Outcome,
    pub data: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import {
    #[serde(rename = "app_id")]
    pub app: String,
    #[serde(rename = "app_secret")]
    pub secret: String,
}

impl Drop for Import {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct State {
    pub version: u8,
    pub app: String,
    pub secret: String,
    pub credential: Option<Credential>,
    pub identity: Option<Identity>,
}

impl Drop for State {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct Credential {
    pub token: String,
    pub expires: i64,
}

impl Drop for Credential {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

impl State {
    pub fn token(&self) -> Result<&str, Error> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            Error::new(
                Class::Authentication,
                "Feishu token absent; explicitly run auth renew, then auth identity",
            )
        })?;
        if credential.expires <= now() + 30 {
            return Err(Error::new(
                Class::Expired,
                "Feishu token expired or near expiry; explicitly renew, then verify identity",
            ));
        }
        visible(&credential.token, 8192)?;
        Ok(&credential.token)
    }

    pub fn bound(&self) -> Result<Identity, Error> {
        let token = self.token()?;
        self.identity
            .as_ref()
            .filter(|id| id.app == self.app && id.stamp == stamp(token) && id.observed > 0)
            .filter(|id| identifier(&id.bot, "ou_").is_ok())
            .cloned()
            .ok_or_else(|| Error::new(Class::Binding, "Feishu identity not verified for current app/token; explicitly run auth identity"))
    }

    pub fn status(&self) -> Status {
        Status {
            app: self.app.clone(),
            expires: self.credential.as_ref().map(|held| held.expires),
            valid: self.token().is_ok(),
            identity: self.bound().ok(),
        }
    }
}

pub(super) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub(super) fn stamp(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub(super) fn identifier(value: &str, prefix: &str) -> Result<(), Error> {
    if !value.starts_with(prefix) || value.len() <= prefix.len() || value.len() > 160 {
        return Err(input());
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(input());
    }
    Ok(())
}

pub(super) fn visible(value: &str, limit: usize) -> Result<(), Error> {
    if value.is_empty() || value.len() > limit || !value.bytes().all(|b| (33..=126).contains(&b)) {
        return Err(input());
    }
    Ok(())
}
