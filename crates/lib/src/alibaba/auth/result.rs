use super::{save, unknown};
use crate::alibaba::{
    Class, Error, Failure, Outcome, api,
    model::{self, Credential, Grant, State},
    secret::Store,
};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use zeroize::Zeroizing;

pub(super) fn settle(store: &Store, state: &mut State) -> Result<Value> {
    let pending = state
        .pending
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba has no pending token operation to recover"))?
        .clone();
    let retained = state.retained.as_ref().ok_or_else(|| unknown("Alibaba pending dispatch has no retained return; never replay it; explicit reauthorization requires discarding local custody"))?;
    if retained.stage != pending.stage {
        bail!("Alibaba retained return does not match the pending operation");
    }
    let bytes = Zeroizing::new(STANDARD.decode(&retained.body).map_err(|_| {
        unknown("Alibaba retained return encoding refused; pending state is preserved")
    })?);
    let body = api::decode(&bytes, Class::Response).map_err(|_| {
        unknown("Alibaba retained token return is malformed; pending state is preserved")
    })?;
    if retained.status != 200 {
        let error = refusal(retained.status, &body.0);
        if error.outcome == Outcome::Rejected {
            state.pending = None;
            state.retained = None;
            save(store, state, true)?;
        }
        return Err(error.into());
    }
    apply(state, &body.0, &pending).map_err(|_| unknown("Alibaba token return is held privately but its fields or lifetime are invalid; use auth recover on the same return, never replay dispatch"))?;
    state.pending = None;
    state.retained = None;
    save(store, state, true)?;
    Ok(state.receipt())
}

fn refusal(status: u16, body: &Value) -> Error {
    let code = body.get("error").and_then(Value::as_str);
    if let Some(code) = body.get("Code").and_then(Value::as_str) {
        let request = body.get("RequestId").and_then(Value::as_str);
        return Error::remote(Failure::acs(status, Some(code), request));
    }
    let request = body
        .get("requestId")
        .or_else(|| body.get("request_id"))
        .and_then(Value::as_str);
    let fault = Failure::oauth(status, code).request(request);
    let rejected = matches!(status, 400 | 401 | 403) && fault.code.is_some();
    let mut error = Error::remote(fault);
    if !rejected {
        error.outcome = Outcome::Unknown;
    }
    error
}

pub(super) fn apply(state: &mut State, value: &Value, pending: &model::Pending) -> Result<()> {
    if pending.stage == model::OIDC {
        super::source::apply(state, value)?;
    } else if pending.stage == model::ROLE {
        super::federation::apply(state, value, pending)?;
    } else if pending.stage == model::STS {
        let expiration = model::field(value, "expiration")?;
        let expires = time::OffsetDateTime::parse(
            expiration,
            &time::format_description::well_known::Rfc3339,
        )?
        .unix_timestamp();
        let expires = u64::try_from(expires)?;
        if expires <= model::now() {
            bail!("Alibaba STS return expired");
        }
        let credential = Credential {
            id: model::field(value, "accessKeyId")?.into(),
            key: model::field(value, "accessKeySecret")?.into(),
            token: model::field(value, "securityToken")?.into(),
            expires,
            observed: pending.started,
        };
        state.credential = Some(credential);
    } else if matches!(pending.stage.as_str(), model::CODE | model::REFRESH) {
        if !model::field(value, "token_type")?.eq_ignore_ascii_case("Bearer") {
            bail!("Alibaba token type refused");
        }
        let lifetime = value
            .get("expires_in")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("Alibaba token lifetime missing"))?;
        let expires = pending
            .started
            .checked_add(lifetime)
            .ok_or_else(|| anyhow::anyhow!("Alibaba token lifetime overflow"))?;
        if expires <= model::now() {
            bail!("Alibaba OAuth return expired");
        }
        let grant = Grant {
            access: model::field(value, "access_token")?.into(),
            refresh: model::field(value, "refresh_token")?.into(),
            expires,
            observed: pending.started,
        };
        state.oauth = Some(grant);
        if pending.stage == model::CODE {
            state.intent = None;
        }
    } else {
        bail!("Alibaba token operation stage refused");
    }
    state.identity = None;
    Ok(())
}
