use super::{Class, Failure, Identity, api, model, secret::Store};
use anyhow::{Result, bail};
use serde_json::Value;
use std::path::Path;

pub struct Client {
    store: Store,
    account: String,
}

impl Client {
    pub fn ram(
        &mut self,
        principal: &str,
        request: &super::ram::Request,
    ) -> Result<super::ram::Receipt, super::Error> {
        use super::{Error, Outcome};
        let refused = |fault| Error {
            fault,
            outcome: Outcome::Unsent,
        };
        let state = self
            .store
            .read()
            .map_err(|error| refused(Failure::capture(error, Class::Custody)))?;
        if state.pending.is_some() || state.retained.is_some() {
            return Err(refused(Failure::new(
                Class::Binding,
                "Alibaba token operation is unresolved; recover before resource access",
            )));
        }
        let expired = state
            .credential
            .as_ref()
            .is_some_and(|held| held.expires <= model::now());
        let class = if expired {
            Class::Expired
        } else {
            Class::Authentication
        };
        let credential = state
            .credential()
            .map_err(|error| refused(Failure::capture(error, class)))?;
        let identity = state.identity.as_ref().ok_or_else(|| {
            refused(Failure::new(
                Class::Binding,
                "Alibaba resource access requires separate explicit caller verification",
            ))
        })?;
        let observed = (
            identity.account.as_str(),
            identity.principal.as_str(),
            identity.stamp.as_str(),
        );
        let stamp = model::stamp(credential);
        let expected = (self.account.as_str(), principal, stamp.as_str());
        if principal.is_empty() || observed != expected {
            return Err(refused(Failure::new(
                Class::Binding,
                "Alibaba resource caller differs from the exact declared account and principal",
            )));
        }
        super::ram::send(credential, identity, request)
    }

    pub fn open(path: &Path, account: &str) -> std::result::Result<Self, Failure> {
        number(account).map_err(|error| Failure::capture(error, Class::Input))?;
        let store = Store::open(path).map_err(|error| Failure::capture(error, Class::Custody))?;
        store
            .read()
            .map_err(|error| Failure::capture(error, Class::Custody))?;
        Ok(Self {
            store,
            account: account.into(),
        })
    }

    pub fn account(&self) -> &str {
        &self.account
    }

    pub fn role(&mut self) -> std::result::Result<super::Role, Failure> {
        let mut state = self
            .store
            .read()
            .map_err(|e| Failure::capture(e, Class::Custody))?;
        if state.pending.is_some() || state.retained.is_some() {
            return Err(Failure::new(
                Class::Binding,
                "Alibaba token operation remains unresolved",
            ));
        }
        let held = state
            .federation
            .as_ref()
            .ok_or_else(|| Failure::new(Class::Binding, "Alibaba federated custody is absent"))?;
        if held.binding.account != self.account {
            return Err(Failure::new(
                Class::Binding,
                "Alibaba role owning account differs from client",
            ));
        }
        let credential = state
            .credential()
            .map_err(|e| Failure::capture(e, Class::Authentication))?;
        let response = api::identity(credential, &api::agent())
            .map_err(|e| Failure::capture(e, Class::Response))?;
        let identity = super::auth::role::decode(&response.0, held, &model::stamp(credential))
            .map_err(|e| Failure::capture(e, Class::Binding))?;
        state
            .federation
            .as_mut()
            .expect("checked federation")
            .identity = Some(identity.clone());
        self.store
            .save(&state)
            .map_err(|e| Failure::capture(e, Class::Custody))?;
        Ok(identity)
    }

    pub fn identity(&mut self) -> std::result::Result<Identity, Failure> {
        let mut state = self
            .store
            .read()
            .map_err(|error| Failure::capture(error, Class::Custody))?;
        if state.pending.is_some() || state.retained.is_some() {
            return Err(Failure::new(
                Class::Binding,
                "Alibaba token operation is unresolved; recover or explicitly discard local custody before caller verification",
            ));
        }
        if state.federation.is_some() {
            return Err(Failure::new(
                Class::Binding,
                "Federated custody requires explicit role verification",
            ));
        }
        let class = if state
            .credential
            .as_ref()
            .is_some_and(|held| held.expires <= model::now())
        {
            Class::Expired
        } else {
            Class::Authentication
        };
        let credential = state
            .credential()
            .map_err(|error| Failure::capture(error, class))?;
        let response = api::identity(credential, &api::agent())
            .map_err(|error| Failure::capture(error, Class::Response))?;
        let identity = decode(&response.0, &self.account, &model::stamp(credential))
            .map_err(|error| Failure::capture(error, Class::Binding))?;
        state.identity = Some(identity.clone());
        self.store
            .save(&state)
            .map_err(|error| Failure::capture(error, Class::Custody))?;
        Ok(identity)
    }
}

pub(super) fn decode(value: &Value, account: &str, stamp: &str) -> Result<Identity> {
    let identity = Identity {
        account: model::field(value, "AccountId")?.into(),
        principal: model::field(value, "PrincipalId")?.into(),
        user: model::field(value, "UserId")?.into(),
        kind: model::field(value, "IdentityType")?.into(),
        arn: model::field(value, "Arn")?.into(),
        stamp: stamp.into(),
        observed: model::now(),
        request: model::field(value, "RequestId")?.into(),
    };
    identity.validate()?;
    if identity.kind != "RAMUser" || identity.account != account {
        bail!("Alibaba caller is root, unsupported or belongs to a different owning account");
    }
    for id in [&identity.account, &identity.principal, &identity.user] {
        number(id)?;
    }
    let prefix = format!("acs:ram::{account}:user/");
    let name = identity.arn.strip_prefix(&prefix).ok_or_else(|| {
        anyhow::anyhow!("Alibaba ARN owner or identity kind differs from the caller")
    })?;
    if name.is_empty() || identity.arn.len() > 512 || !identity.arn.is_ascii() {
        bail!("Alibaba RAM user ARN is malformed");
    }
    if identity.request.len() > 128
        || !identity
            .request
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!("Alibaba identity RequestId is malformed");
    }
    Ok(identity)
}

fn number(value: &str) -> Result<()> {
    if value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || value.parse::<u64>().ok().is_none_or(|number| number == 0)
    {
        bail!(
            "Alibaba account and native RAM user identifiers must be explicit positive numeric strings"
        );
    }
    Ok(())
}
