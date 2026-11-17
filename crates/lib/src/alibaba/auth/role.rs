use super::super::model;
use super::Binding;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Federation {
    pub binding: Binding,
    pub token: Option<Token>,
    pub identity: Option<Identity>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Token {
    pub value: String,
    pub expires: u64,
    pub issued: u64,
}

impl Drop for Token {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub account: String,
    pub principal: String,
    pub role: String,
    pub arn: String,
    pub stamp: String,
    pub observed: u64,
    pub request: String,
}

impl Federation {
    pub(in crate::alibaba) fn check(&self, credential: Option<&model::Credential>) -> Result<()> {
        self.binding.validate()?;
        if let Some(token) = &self.token {
            model::text(&token.value)?;
            if token.issued == 0 || token.expires <= token.issued {
                bail!("Ensign token expiry refused");
            }
        }
        if let Some(identity) = &self.identity {
            let credential = credential.ok_or_else(|| {
                anyhow::anyhow!("Alibaba role identity has no signing credential")
            })?;
            let observed = (&identity.account, &identity.role, identity.arn.as_str());
            let arn = self.binding.caller();
            let stamp = model::stamp(credential);
            let expected = (&self.binding.account, &self.binding.id, arn.as_str());
            if observed != expected || identity.stamp != stamp {
                bail!(
                    "Alibaba role identity no longer matches its declared binding and credential"
                );
            }
            for value in [&identity.principal, &identity.request] {
                model::text(value)?;
            }
        }
        Ok(())
    }
}

pub(in crate::alibaba) fn decode(
    value: &serde_json::Value,
    held: &Federation,
    stamp: &str,
) -> Result<Identity> {
    let identity = Identity {
        account: model::field(value, "AccountId")?.into(),
        principal: model::field(value, "PrincipalId")?.into(),
        role: model::field(value, "RoleId")?.into(),
        arn: model::field(value, "Arn")?.into(),
        stamp: stamp.into(),
        observed: model::now(),
        request: model::field(value, "RequestId")?.into(),
    };
    let observed = (
        identity.account.as_str(),
        identity.role.as_str(),
        identity.arn.as_str(),
    );
    let arn = held.binding.caller();
    let expected = (
        held.binding.account.as_str(),
        held.binding.id.as_str(),
        arn.as_str(),
    );
    if observed != expected || model::field(value, "IdentityType")? != "AssumedRoleUser" {
        bail!("Alibaba caller is not the exact declared owner, stable RoleId and role session");
    }
    if identity.request.len() > 128
        || !identity
            .request
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || v == b'-')
    {
        bail!("Alibaba role identity request receipt refused");
    }
    Ok(identity)
}
