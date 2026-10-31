use super::{
    super::{
        Class, Error, Identity,
        fault::response,
        model::{Credential, State, identifier, now, stamp, visible},
        wire,
    },
    session::{Grant, scopes},
};
use crate::http::wire as http;
use serde_json::{Value, json};
use zeroize::Zeroizing;

pub(crate) fn exchange(agent: &http::Client, state: &State) -> Result<Grant, Error> {
    let user = state.user.as_ref().ok_or_else(response)?;
    let device = user.device.as_ref().ok_or_else(response)?;
    let form = Zeroizing::new(
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "urn:ietf:params:oauth:grant-type:device_code")
            .append_pair("client_id", &state.app)
            .append_pair("client_secret", &state.secret)
            .append_pair("device_code", &device.secret)
            .finish(),
    );
    let started = now();
    let record = wire::oauth(
        agent,
        http::Request {
            method: "POST",
            url: "https://accounts.feishu.cn/oauth/v3/token",
            headers: &[("Content-Type", "application/x-www-form-urlencoded")],
            body: Some(form.as_bytes()),
        },
    )?;
    decode(&record.body.0, started, None)
}

pub(crate) fn renew(agent: &http::Client, state: &State) -> Result<Grant, Error> {
    let grant = state
        .user
        .as_ref()
        .and_then(|u| u.grant.as_ref())
        .ok_or_else(response)?;
    let body = wire::Body(json!({"grant_type":"refresh_token", "client_id":state.app,
        "client_secret":state.secret,"refresh_token":grant.refresh}));
    let bytes = Zeroizing::new(serde_json::to_vec(&body.0).map_err(|_| response())?);
    let started = now();
    let record = wire::oauth(
        agent,
        http::Request {
            method: "POST",
            url: "https://accounts.feishu.cn/oauth/v3/token",
            headers: &[("Content-Type", "application/json")],
            body: Some(&bytes),
        },
    )?;
    decode(&record.body.0, started, Some(grant))
}

pub(super) fn decode(
    value: &Value,
    started: i64,
    previous: Option<&Grant>,
) -> Result<Grant, Error> {
    if value["token_type"].as_str() != Some("Bearer") {
        return Err(response());
    }
    let token = value["access_token"].as_str().ok_or_else(response)?;
    visible(token, 8192).map_err(|_| response())?;
    let ttl = value["expires_in"]
        .as_i64()
        .filter(|n| (60..=86400).contains(n))
        .ok_or_else(response)?;
    let refresh = match value.get("refresh_token") {
        Some(Value::String(value)) => value.as_str(),
        None => previous.map(|g| g.refresh.as_str()).ok_or_else(response)?,
        _ => return Err(response()),
    };
    visible(refresh, 8192).map_err(|_| response())?;
    let expires = match value.get("refresh_token_expires_in") {
        Some(value) => {
            started
                + value
                    .as_i64()
                    .filter(|n| (60..=366 * 86400).contains(n))
                    .ok_or_else(response)?
        }
        None => previous.map(|g| g.expires).ok_or_else(response)?,
    };
    let scopes = match value.get("scope") {
        Some(Value::String(scope)) => scopes(
            &scope
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>(),
        )
        .map_err(|_| response())?,
        None => previous.map(|g| g.scopes.clone()).ok_or_else(response)?,
        _ => return Err(response()),
    };
    Ok(Grant {
        credential: Credential {
            token: token.into(),
            expires: started + ttl,
        },
        refresh: refresh.into(),
        expires,
        scopes,
    })
}

pub(crate) fn identity(agent: &http::Client, state: &State) -> Result<Identity, Error> {
    let user = state.user.as_ref().ok_or_else(response)?;
    let token = user.token()?;
    let header = Zeroizing::new(format!("Bearer {token}"));
    let record = wire::invoke(
        agent,
        http::Request {
            method: "GET",
            url: "https://open.feishu.cn/open-apis/authen/v1/user_info",
            headers: &[("Authorization", &header)],
            body: None,
        },
    )?;
    let found = record.body.0["data"]["open_id"]
        .as_str()
        .ok_or_else(response)?;
    identifier(found, "ou_").map_err(|_| response())?;
    if found != user.expected {
        return Err(Error::new(
            Class::Binding,
            "Feishu authorized user does not match the expected user; discard local user session and authorize the correct person",
        ));
    }
    Ok(Identity {
        app: state.app.clone(),
        bot: String::new(),
        user: Some(found.into()),
        observed: now(),
        stamp: stamp(token),
    })
}
