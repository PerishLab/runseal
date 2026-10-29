use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::Zeroize;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::tencent) struct State {
    pub version: u32,
    pub intent: Option<Intent>,
    pub oauth: Option<Grant>,
    pub credential: Option<Credential>,
    pub identity: Option<Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::tencent) struct Intent {
    pub state: String,
    pub deadline: u64,
    pub consumed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::tencent) struct Grant {
    pub subject: String,
    pub access: String,
    pub refresh: String,
    pub expires: u64,
    pub site: String,
}

impl Drop for Grant {
    fn drop(&mut self) {
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::tencent) struct Credential {
    pub id: String,
    pub key: String,
    pub token: Option<String>,
    pub expires: Option<u64>,
}

impl Drop for Credential {
    fn drop(&mut self) {
        self.key.zeroize();
        self.id.zeroize();
        self.token.zeroize();
    }
}

impl State {
    pub fn fresh() -> Self {
        Self {
            version: 1,
            ..Self::default()
        }
    }

    pub fn credential(&self) -> Result<&Credential> {
        let held = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Tencent credentials are absent; authorize and complete login first")
        })?;
        if held.expires.is_some_and(|end| end <= now()) {
            bail!(
                "Tencent credentials expired; explicitly refresh the authorization token if needed, renew credentials, then verify identity"
            );
        }
        text(&held.id)?;
        text(&held.key)?;
        if let Some(token) = &held.token {
            text(token)?;
        }
        Ok(held)
    }

    pub fn receipt(&self) -> Value {
        json!({"authorized": self.oauth.is_some(), "credentialPresent": self.credential.is_some(), "credentialExpiresAt": self.credential.as_ref().and_then(|held| held.expires), "oauthExpiresAt": self.oauth.as_ref().map(|held| held.expires), "identity": self.identity, "pending": self.intent.as_ref().is_some_and(|held| !held.consumed && held.deadline > now())})
    }
}

pub(in crate::tencent) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(in crate::tencent) fn text(value: &str) -> Result<&str> {
    if value.is_empty() || value.len() > 16384 || value.chars().any(char::is_control) {
        bail!("Tencent data refused");
    }
    Ok(value)
}

pub(in crate::tencent) fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    text(
        value
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Tencent response refused"))?,
    )
}

pub(in crate::tencent) fn stamp(credential: &Credential) -> String {
    use sha2::{Digest, Sha256};
    let text = zeroize::Zeroizing::new(format!(
        "{}\n{}",
        credential.id,
        credential.token.as_deref().unwrap_or_default()
    ));
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
