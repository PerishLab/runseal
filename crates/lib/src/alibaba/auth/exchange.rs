use super::{available, result, save, unknown};
use crate::alibaba::{
    model::{self, Pending, Retained, State},
    oauth,
    secret::Store,
};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use zeroize::Zeroizing;

pub(super) fn refresh(store: &Store) -> Result<Value> {
    let mut state = store.read()?;
    available(&state)?;
    let grant = state
        .oauth
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba OAuth authorization is absent"))?;
    let body = oauth::form(&[
        ("grant_type", "refresh_token"),
        ("client_id", oauth::CLIENT),
        ("refresh_token", &grant.refresh),
    ]);
    perform(store, &mut state, model::REFRESH, &body)?;
    result::settle(store, &mut state)
}

pub(super) fn renew(store: &Store) -> Result<Value> {
    let mut state = store.read()?;
    available(&state)?;
    let grant = state
        .oauth
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba OAuth authorization is absent"))?;
    if grant.expires <= model::now() {
        bail!("Alibaba OAuth access expired; explicitly refresh before requesting STS credentials");
    }
    let access = Zeroizing::new(grant.access.clone());
    perform(store, &mut state, model::STS, &access)?;
    result::settle(store, &mut state)
}

pub(super) fn perform(store: &Store, state: &mut State, stage: &str, payload: &str) -> Result<()> {
    state.identity = None;
    state.credential = None;
    state.pending = Some(Pending {
        stage: stage.into(),
        started: model::now(),
    });
    save(store, state, false)?;
    let response = if stage == model::STS {
        oauth::exchange(&oauth::agent(), payload)
    } else {
        oauth::token(&oauth::agent(), payload)
    };
    let (status, body) = response.map_err(|_| unknown("Alibaba token dispatch result was not observed; the saved pending intent forbids replay; inspect status before recovery"))?;
    state.retained = Some(Retained {
        stage: stage.into(),
        status,
        body: STANDARD.encode(&body),
    });
    save(store, state, true)
}
