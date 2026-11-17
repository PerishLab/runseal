use super::{available, exchange, result, save};
use crate::alibaba::{
    api::sign,
    model::{self, Intent, State},
    oauth,
    secret::{self, Store},
};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use zeroize::Zeroizing;

pub(super) fn begin(store: &Store) -> Result<Value> {
    let exists = store.exists()?;
    if exists {
        let state = store.read()?;
        available(&state)?;
        if state.oauth.is_some() || state.credential.is_some() {
            bail!(
                "Alibaba authorization already exists; explicitly clear local custody before starting another login"
            );
        }
        if state
            .intent
            .as_ref()
            .is_some_and(|intent| !intent.consumed && intent.deadline > model::now())
        {
            bail!(
                "Alibaba authorization is already pending; use its existing URL or explicitly clear local custody before starting again"
            );
        }
    }
    let (nonce, digest) = oauth::state()?;
    let verifier = oauth::verifier()?;
    let deadline = model::now()
        .checked_add(600)
        .ok_or_else(|| anyhow::anyhow!("Alibaba authorization time refused"))?;
    let url = oauth::authorize(&nonce, &oauth::challenge(&verifier))?;
    let mut state = State::fresh();
    state.intent = Some(Intent {
        state: digest,
        verifier: verifier.to_string(),
        deadline,
        consumed: false,
        redirect: oauth::REDIRECT.into(),
    });
    if exists {
        save(store, &state, false)?;
    } else {
        store.create(&state)?;
    }
    Ok(
        json!({"authorizationUrl":url,"expiresAt":deadline,"callbackUri":oauth::REDIRECT,"application":"Alibaba Cloud official CLI OAuth application, native CN protocol","delivery":"Complete browser login/MFA/consent, save the full redirected callback URL in a private file, then auth complete --response-file PATH. Do not paste the callback into chat."}),
    )
}

pub(super) fn complete(store: &Store, path: &Path) -> Result<Value> {
    let mut state = store.read()?;
    available(&state)?;
    let intent = state
        .intent
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba authorization was not begun"))?;
    if intent.consumed || intent.deadline <= model::now() || intent.redirect != oauth::REDIRECT {
        bail!("Alibaba authorization expired, consumed or differs from the registered redirect");
    }
    let input = secret::read(path)?;
    let digest = sign::hash(&input);
    let raw = std::str::from_utf8(&input)
        .map_err(|_| anyhow::anyhow!("Alibaba private callback encoding refused"))?;
    let code = Zeroizing::new(oauth::callback(raw, &intent.state)?);
    let body = oauth::form(&[
        ("grant_type", "authorization_code"),
        ("client_id", oauth::CLIENT),
        ("code", &code),
        ("redirect_uri", oauth::REDIRECT),
        ("code_verifier", &intent.verifier),
    ]);
    state.intent.as_mut().expect("checked intent").consumed = true;
    exchange::perform(store, &mut state, model::CODE, &body)?;
    let unchanged = secret::read(path).is_ok_and(|bytes| sign::hash(&bytes) == digest);
    if !unchanged || std::fs::remove_file(path).is_err() {
        return Err(super::unknown("Alibaba token return is held privately but callback consumption failed; inspect local input and use auth recover without replay").into());
    }
    result::settle(store, &mut state)
}
