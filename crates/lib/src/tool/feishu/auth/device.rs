use super::{
    super::{
        Error,
        fault::response,
        model::{State, now, visible},
        wire,
    },
    session::{Challenge, Device, scopes},
};
use crate::http::wire as http;
use base64::{Engine, engine::general_purpose::STANDARD};
use zeroize::Zeroizing;

pub(crate) fn begin(
    agent: &http::Client,
    state: &State,
    requested: &[String],
) -> Result<Device, Error> {
    let scopes = scopes(requested)?;
    let pair = Zeroizing::new(format!("{}:{}", state.app, state.secret));
    let basic = Zeroizing::new(STANDARD.encode(pair.as_bytes()));
    let header = Zeroizing::new(format!("Basic {}", &*basic));
    let form = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", &state.app)
        .append_pair("scope", &scopes.join(" "))
        .finish();
    let started = now();
    let result = wire::oauth(
        agent,
        http::Request {
            method: "POST",
            url: "https://accounts.feishu.cn/oauth/v1/device_authorization",
            headers: &[
                ("Authorization", &header),
                ("Content-Type", "application/x-www-form-urlencoded"),
            ],
            body: Some(form.as_bytes()),
        },
    )?;
    decode(&result.body.0, started, scopes).map_err(|mut error| {
        error.status = Some(200);
        error.request = result.request.clone();
        error
    })
}

pub(super) fn decode(
    value: &serde_json::Value,
    started: i64,
    scopes: Vec<String>,
) -> Result<Device, Error> {
    let secret = value["device_code"].as_str().ok_or_else(response)?;
    visible(secret, 8192).map_err(|_| response())?;
    let code = value["user_code"].as_str().ok_or_else(response)?;
    visible(code, 128).map_err(|_| response())?;
    let verification = value["verification_uri"].as_str().ok_or_else(response)?;
    check(verification, secret)?;
    let ttl = value["expires_in"]
        .as_i64()
        .filter(|n| (30..=3600).contains(n))
        .ok_or_else(response)?;
    let interval = value["interval"]
        .as_i64()
        .filter(|n| (1..=60).contains(n))
        .ok_or_else(response)?;
    Ok(Device {
        secret: secret.into(),
        challenge: Challenge {
            verification: verification.into(),
            code: code.into(),
            expires: started + ttl,
            interval,
        },
        next: started + interval,
        scopes,
    })
}

fn check(verification: &str, secret: &str) -> Result<(), Error> {
    let uri = url::Url::parse(verification).map_err(|_| response())?;
    if uri.scheme() != "https" || uri.host_str() != Some("accounts.feishu.cn") {
        return Err(response());
    }
    if !uri.username().is_empty() || uri.password().is_some() || uri.port().is_some() {
        return Err(response());
    }
    if uri.fragment().is_some() || verification.len() > 2048 {
        return Err(response());
    }
    if uri.query().is_some() {
        let pairs: Vec<_> = uri.query_pairs().collect();
        if pairs.len() != 1 {
            return Err(response());
        }
        let (key, value) = &pairs[0];
        if key != "flow_id" || value == secret {
            return Err(response());
        }
        visible(value, 1024).map_err(|_| response())?;
    }
    Ok(())
}
