use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::time::Duration;
use zeroize::Zeroizing;

use crate::http::wire;

pub(super) const CLIENT: &str = "4038181954557748008";
pub(super) const AUTHORIZE: &str = "https://signin.aliyun.com/oauth2/v1/auth";
pub(super) const TOKEN: &str = "https://oauth.aliyun.com/v1/token";
pub(super) const EXCHANGE: &str = "https://oauth.aliyun.com/v1/exchange";
pub(super) const REDIRECT: &str = "http://127.0.0.1:12345/cli/callback";

#[derive(PartialEq)]
struct Address<'a> {
    scheme: &'a str,
    host: Option<&'a str>,
    port: Option<u16>,
    path: &'a str,
}

impl<'a> Address<'a> {
    fn from(url: &'a url::Url) -> Self {
        Self {
            scheme: url.scheme(),
            host: url.host_str(),
            port: url.port(),
            path: url.path(),
        }
    }
}

pub(super) fn agent() -> wire::Client {
    wire::Client::new(wire::Policy {
        redirects: 0,
        overall: Some(Duration::from_secs(20)),
        connect: Some(Duration::from_secs(5)),
        limit: super::model::BODY as u64,
    })
}

pub(super) fn verifier() -> Result<Zeroizing<String>> {
    let mut random = [0u8; 64];
    getrandom::fill(&mut random)
        .map_err(|_| anyhow::anyhow!("Alibaba authorization entropy unavailable"))?;
    Ok(Zeroizing::new(URL_SAFE_NO_PAD.encode(random)))
}

pub(super) fn challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(super) fn state() -> Result<(String, String)> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random)
        .map_err(|_| anyhow::anyhow!("Alibaba authorization entropy unavailable"))?;
    let value = URL_SAFE_NO_PAD.encode(random);
    Ok((value.clone(), super::api::sign::hash(value.as_bytes())))
}

pub(super) fn authorize(state: &str, challenge: &str) -> Result<String> {
    let mut url = url::Url::parse(AUTHORIZE)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT)
        .append_pair("redirect_uri", REDIRECT)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url.into())
}

pub(super) fn callback(raw: &str, expected: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 8192 {
        bail!("Alibaba callback is empty or oversized");
    }
    let url = url::Url::parse(raw).map_err(|_| anyhow::anyhow!("Alibaba callback is not a URL"))?;
    let redirect = url::Url::parse(REDIRECT).expect("static redirect URI");
    let actual = Address::from(&url);
    let registered = Address::from(&redirect);
    let private = !url.username().is_empty() || url.password().is_some();
    if actual != registered || private || url.fragment().is_some() {
        bail!("Alibaba callback address differs from the fixed registered redirect");
    }
    let mut code = None;
    let mut state = None;
    let mut denied = false;
    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "state" => single(&mut state, &value)?,
            "code" => single(&mut code, &value)?,
            "error" => denied = true,
            _ => (),
        }
    }
    if state
        .as_deref()
        .map(|value| super::api::sign::hash(value.as_bytes()))
        .as_deref()
        != Some(expected)
    {
        bail!("Alibaba callback state does not match this authorization");
    }
    if denied {
        bail!(
            "Alibaba authorization was denied or failed at the provider; begin a new authorization"
        );
    }
    let code = code.unwrap_or_default();
    if code.trim().is_empty() || code.chars().any(char::is_control) {
        bail!("Alibaba callback carried no usable authorization code");
    }
    Ok(code)
}

fn single(held: &mut Option<String>, value: &str) -> Result<()> {
    if held.replace(value.to_owned()).is_some() {
        bail!("Alibaba callback repeated a bound parameter");
    }
    Ok(())
}

pub(super) fn form(fields: &[(&str, &str)]) -> Zeroizing<String> {
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in fields {
        encoded.append_pair(name, value);
    }
    Zeroizing::new(encoded.finish())
}

pub(super) fn token(agent: &wire::Client, body: &str) -> Result<(u16, Zeroizing<Vec<u8>>)> {
    let response = agent
        .send(wire::Request {
            method: "POST",
            url: TOKEN,
            headers: &[("Content-Type", "application/x-www-form-urlencoded")],
            body: Some(body.as_bytes()),
        })
        .map_err(|_| anyhow::anyhow!("Alibaba token transport produced no observed response"))?;
    let status = response.status;
    let payload = response
        .collect()
        .map_err(|_| anyhow::anyhow!("Alibaba token response was unreadable"))?;
    Ok((status, payload.body))
}

pub(super) fn exchange(agent: &wire::Client, access: &str) -> Result<(u16, Zeroizing<Vec<u8>>)> {
    let bearer = Zeroizing::new(format!("Bearer {access}"));
    let response = agent
        .send(wire::Request {
            method: "POST",
            url: EXCHANGE,
            headers: &[
                ("Authorization", bearer.as_str()),
                ("Content-Type", "application/json"),
            ],
            body: None,
        })
        .map_err(|_| {
            anyhow::anyhow!("Alibaba credential transport produced no observed response")
        })?;
    let status = response.status;
    let payload = response
        .collect()
        .map_err(|_| anyhow::anyhow!("Alibaba credential response was unreadable"))?;
    Ok((status, payload.body))
}

#[cfg(test)]
mod tests {
    use super::{REDIRECT, callback, challenge, form, verifier};

    #[test]
    fn pkce() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let verifier = verifier().unwrap();
        assert!((43..=128).contains(&verifier.len()));
        assert!(
            verifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'_' | b'.' | b'~'))
        );
    }

    #[test]
    fn returned() {
        let state = "fixture-state";
        let raw = format!("{REDIRECT}?code=fixture-code&state={state}");
        let expected = crate::alibaba::api::sign::hash(state.as_bytes());
        assert_eq!(callback(&raw, &expected).unwrap(), "fixture-code");
        for rejected in [
            format!("http://127.0.0.1:12345/other?code=c&state={state}"),
            format!("https://127.0.0.1:12345/cli/callback?code=c&state={state}"),
            format!("http://127.0.0.1:12346/cli/callback?code=c&state={state}"),
            format!("http://localhost:12345/cli/callback?code=c&state={state}"),
            format!("{REDIRECT}?code=c&state=other"),
            format!("{REDIRECT}?state={state}"),
            format!("{REDIRECT}?code=c&code=d&state={state}"),
            format!("{REDIRECT}?code=c&state={state}&state={state}"),
            format!("{REDIRECT}?error=access_denied&state={state}"),
            format!("{REDIRECT}?code=&state={state}"),
            format!("user@127.0.0.1:12345/cli/callback?code=c&state={state}"),
            format!("{REDIRECT}?code=c&state={state}#fragment"),
        ] {
            assert!(callback(&rejected, &expected).is_err(), "{rejected}");
        }
    }

    #[test]
    fn encoded() {
        assert_eq!(
            form(&[
                ("grant_type", "authorization_code"),
                ("redirect_uri", REDIRECT)
            ])
            .as_str(),
            "grant_type=authorization_code&redirect_uri=http%3A%2F%2F127.0.0.1%3A12345%2Fcli%2Fcallback"
        );
    }
}
