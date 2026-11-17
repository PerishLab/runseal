use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::{Zeroize, Zeroizing};

pub(super) const SITE: &str = "CN";
pub(super) const CODE: &str = "code-exchange";
pub(super) const REFRESH: &str = "token-refresh";
pub(super) const STS: &str = "sts-exchange";
pub(super) const STATE: usize = 2 * 1024 * 1024;
pub(super) const BODY: usize = 256 * 1024;
pub(super) const RETAINED: usize = BODY / 3 * 4 + 8;

pub(super) fn stage(value: &str) -> Result<()> {
    if !matches!(value, CODE | REFRESH | STS) {
        bail!("Alibaba pending stage refused");
    }
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct State {
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

impl State {
    pub(super) fn fresh() -> Self {
        Self {
            version: 1,
            site: SITE.into(),
            ..Self::default()
        }
    }

    pub(super) fn check(&self) -> Result<()> {
        if self.version != 1 || self.site != SITE {
            bail!("Alibaba state version or site refused");
        }
        if let Some(intent) = &self.intent {
            for value in [&intent.state, &intent.verifier, &intent.redirect] {
                text(value)?;
            }
        }
        if let Some(grant) = &self.oauth {
            text(&grant.access)?;
            text(&grant.refresh)?;
        }
        if let Some(credential) = &self.credential {
            text(&credential.id)?;
            text(&credential.key)?;
            text(&credential.token)?;
        }
        if let Some(identity) = &self.identity {
            identity.validate()?;
        }
        if let Some(pending) = &self.pending {
            stage(&pending.stage)?;
            if pending.started == 0 {
                bail!("Alibaba pending stage refused");
            }
        }
        if let Some(retained) = &self.retained {
            stage(&retained.stage)?;
            if self
                .pending
                .as_ref()
                .is_none_or(|pending| pending.stage != retained.stage)
            {
                bail!("Alibaba retained response has no matching pending stage");
            }
            if retained.body.len() > RETAINED {
                bail!("Alibaba retained response exceeds the private state allowance");
            }
            let decoded = Zeroizing::new(
                base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    retained.body.as_bytes(),
                )
                .map_err(|_| anyhow::anyhow!("Alibaba retained response encoding refused"))?,
            );
            if decoded.len() > BODY {
                bail!("Alibaba retained response exceeds the response allowance");
            }
        }
        if let Some(identity) = &self.identity {
            let credential = self.credential.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Alibaba verified identity requires its signing credential")
            })?;
            if identity.stamp != stamp(credential) {
                bail!("Alibaba verified identity no longer matches the stored signing material");
            }
        }
        Ok(())
    }

    pub(super) fn credential(&self) -> Result<&Credential> {
        let held = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Alibaba STS credentials are absent; renew explicitly")
        })?;
        if held.expires <= now() {
            bail!(
                "Alibaba STS credentials expired; explicitly refresh the OAuth token if needed, renew, then verify identity"
            );
        }
        text(&held.id)?;
        text(&held.key)?;
        text(&held.token)?;
        Ok(held)
    }

    pub(super) fn receipt(&self) -> Value {
        json!({
            "site": SITE,
            "authorized": self.oauth.is_some(),
            "credentialPresent": self.credential.is_some(),
            "oauthExpiresAt": self.oauth.as_ref().map(|held| held.expires),
            "credentialExpiresAt": self.credential.as_ref().map(|held| held.expires),
            "identity": self.identity,
            "pending": self.pending.as_ref().map(|held| json!({"stage": held.stage, "startedAt": held.started, "retainedReturn": self.retained.is_some()})),
            "callback": self.intent.as_ref().map(|held| json!({"redirect": held.redirect, "expiresAt": held.deadline, "consumed": held.consumed})),
        })
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
