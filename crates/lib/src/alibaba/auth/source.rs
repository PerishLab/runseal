use super::super::{
    model::{self, Pending, Retained, State},
    secret::Store,
};
use super::{Binding, save, unknown};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn acquire(store: &Store, home: &Path, binding: &Binding) -> Result<Value> {
    binding.validate()?;
    if !home.is_absolute() {
        bail!("Ensign data directory must be explicitly absolute");
    }
    let mut state = prepare(store, binding)?;
    let client = ensign_client::Client::open(home).map_err(|_| {
        anyhow::anyhow!(
            "Ensign retained login unavailable; sign in explicitly using the Ensign client"
        )
    })?;
    if client.base() != binding.source {
        bail!("Ensign retained endpoint differs from the declared source");
    }
    let discovery = client
        .discovery()
        .map_err(|_| anyhow::anyhow!("Ensign discovery failed"))?;
    if discovery.issuer != binding.issuer
        || discovery.endpoint != format!("{}/token", binding.issuer)
        || discovery.keys != format!("{}/.well-known/jwks.json", binding.issuer)
    {
        bail!("Ensign discovery differs from the independently declared canonical issuer");
    }
    let identity = client
        .identity()
        .map_err(|_| anyhow::anyhow!("Ensign identity lookup failed"))?;
    if identity.sub != binding.subject {
        bail!("Ensign external subject differs from declared identity");
    }
    let (status, keys) = client
        .call("GET", "/.well-known/jwks.json", None)
        .map_err(|_| anyhow::anyhow!("Ensign verification keys unavailable"))?;
    if status != 200 || serde_json::to_vec(&keys)?.len() > 65536 {
        bail!("Ensign verification key response refused");
    }
    state.pending = Some(Pending {
        stage: model::OIDC.into(),
        started: model::now(),
    });
    state.credential = None;
    let federation = state.federation.as_mut().expect("prepared federation");
    federation.token = None;
    federation.identity = None;
    save(store, &state, false)?;
    let response = client.exchange(&binding.audience).map_err(|_| unknown("Ensign exchange outcome is unknown; inspect status and explicitly discard local custody before a new attempt"))?;
    let body = super::super::api::Body(json!({"token": response.token.expose(), "keys":keys}));
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(&body.0)?);
    state.retained = Some(Retained {
        stage: model::OIDC.into(),
        status: 200,
        body: STANDARD.encode(&bytes),
    });
    save(store, &state, true)?;
    super::result::settle(store, &mut state)
}

fn prepare(store: &Store, binding: &Binding) -> Result<State> {
    if store.exists()? {
        let state = store.read()?;
        super::available(&state)?;
        match &state.federation {
            Some(held) if held.binding == *binding => return Ok(state),
            _ => bail!(
                "Alibaba federation requires separate custody; explicitly logout before replacing a binding"
            ),
        }
    }
    let mut state = State::fresh();
    state.version = 2;
    state.federation = Some(super::role::Federation {
        binding: binding.clone(),
        token: None,
        identity: None,
    });
    store.create(&state)?;
    Ok(state)
}

pub(super) fn apply(state: &mut State, value: &Value) -> Result<()> {
    let held = state
        .federation
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Ensign federation binding missing"))?;
    let token = model::field(value, "token")?;
    let claims = super::token::verify(token, &value["keys"], &held.binding)?;
    held.token = Some(super::role::Token {
        value: token.into(),
        expires: claims.exp,
        issued: claims.iat,
    });
    held.identity = None;
    state.credential = None;
    Ok(())
}
