mod state;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::{Zeroize, Zeroizing};

pub(super) const SITE: &str = "CN";
pub(super) const CODE: &str = "code-exchange";
pub(super) const REFRESH: &str = "token-refresh";
pub(super) const OIDC: &str = "ensign-exchange";
pub(super) const ROLE: &str = "oidc-sts";
pub(super) const STS: &str = "sts-exchange";
pub(super) const STATE: usize = 2 * 1024 * 1024;
pub(super) const BODY: usize = 256 * 1024;
pub(super) const RETAINED: usize = BODY / 3 * 4 + 8;

pub(super) fn stage(value: &str) -> Result<()> {
    if !matches!(value, CODE | REFRESH | STS | OIDC | ROLE) {
        bail!("Alibaba pending stage refused");
    }
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub federation: Option<super::auth::role::Federation>,
    pub version: u32,
    pub site: String,
    pub intent: Option<Intent>,
    pub oauth: Option<Grant>,
    pub credential: Option<Credential>,
    pub identity: Option<Identity>,
    pub pending: Option<Pending>,
    pub retained: Option<Retained>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Intent {
    pub state: String,
    pub verifier: String,
    pub deadline: u64,
    pub consumed: bool,
    pub redirect: String,
}

impl Drop for Intent {
    fn drop(&mut self) {
        self.verifier.zeroize();
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Grant {
    pub access: String,
    pub refresh: String,
    pub expires: u64,
    pub observed: u64,
}

impl Drop for Grant {
    fn drop(&mut self) {
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Credential {
    pub id: String,
    pub key: String,
    pub token: String,
    pub expires: u64,
    pub observed: u64,
}

impl Drop for Credential {
    fn drop(&mut self) {
        self.id.zeroize();
        self.key.zeroize();
        self.token.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Pending {
    pub stage: String,
    pub started: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Retained {
    pub stage: String,
    pub status: u16,
    pub body: String,
}

impl Drop for Retained {
    fn drop(&mut self) {
        self.body.zeroize();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub account: String,
    pub principal: String,
    pub user: String,
    pub kind: String,
    pub arn: String,
    pub stamp: String,
    pub observed: u64,
    pub request: String,
}

impl Identity {
    pub(super) fn validate(&self) -> Result<()> {
        for value in [
            &self.account,
            &self.principal,
            &self.user,
            &self.kind,
            &self.arn,
            &self.request,
        ] {
            text(value)?;
        }
        if !self.account.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("Alibaba identity account refused");
        }
        if self.kind != "RAMUser" {
            bail!("Alibaba identity refused: only a RAM user caller is supported");
        }
        let prefix = format!("acs:ram::{}:user/", self.account);
        if !self.arn.starts_with(&prefix) || self.arn.len() == prefix.len() {
            bail!("Alibaba identity ARN does not name a user of the declared account");
        }
        if self.stamp.len() != 64 || !self.stamp.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("Alibaba identity stamp refused");
        }
        Ok(())
    }
}

pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) fn text(value: &str) -> Result<&str> {
    if value.is_empty() || value.len() > 16384 || value.chars().any(char::is_control) {
        bail!("Alibaba data refused");
    }
    Ok(value)
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    text(
        value
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Alibaba response refused"))?,
    )
}

pub(super) fn stamp(credential: &Credential) -> String {
    use sha2::{Digest, Sha256};
    let text = zeroize::Zeroizing::new(format!(
        "{}\n{}\n{}\n{}",
        credential.id, credential.key, credential.token, credential.expires
    ));
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
