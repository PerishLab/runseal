use super::{
    Error, Identity,
    fault::response,
    model::{Credential, State, identifier, now, stamp, visible},
    wire,
};
use crate::http::wire as http;
use serde_json::Value;
use zeroize::Zeroizing;

pub(super) fn renew(agent: &http::Client, state: &mut State) -> Result<(), Error> {
    let form = Zeroizing::new(
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "client_credentials")
            .append_pair("client_id", &state.app)
            .append_pair("client_secret", &state.secret)
            .finish(),
    );
    let started = now();
    let result = wire::invoke(
        agent,
        http::Request {
            method: "POST",
            url: "https://accounts.feishu.cn/oauth/v3/token",
            headers: &[("Content-Type", "application/x-www-form-urlencoded")],
            body: Some(form.as_bytes()),
        },
    )?;
    let credential = credential(&result.body.0, started)?;
    state.credential = Some(credential);
    state.identity = None;
    Ok(())
}

fn credential(value: &Value, started: i64) -> Result<Credential, Error> {
    if value["token_type"].as_str() != Some("Bearer") {
        return Err(response());
    }
    let token = value["access_token"].as_str().ok_or_else(response)?;
    visible(token, 8192).map_err(|_| response())?;
    let ttl = value["expires_in"]
        .as_i64()
        .filter(|n| (60..=86400).contains(n))
        .ok_or_else(response)?;
    Ok(Credential {
        token: token.into(),
        expires: started + ttl,
    })
}

pub(super) fn identity(agent: &http::Client, state: &State) -> Result<Identity, Error> {
    let token = state.token()?;
    let header = Zeroizing::new(format!("Bearer {token}"));
    let result = wire::invoke(
        agent,
        http::Request {
            method: "GET",
            url: "https://open.feishu.cn/open-apis/bot/v3/info/",
            headers: &[("Authorization", &header)],
            body: None,
        },
    )?;
    let bot = result.body.0["bot"]["open_id"]
        .as_str()
        .ok_or_else(response)?;
    identifier(bot, "ou_").map_err(|_| response())?;
    Ok(Identity {
        app: state.app.clone(),
        bot: bot.into(),
        observed: now(),
        stamp: stamp(token),
    })
}

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;
