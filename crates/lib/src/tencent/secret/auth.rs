use super::{
    api,
    command::Auth,
    model::{Credential, Grant, Intent, State, field, now},
    secret::{self, Store},
    sign,
};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use zeroize::{Zeroize, Zeroizing};

const BRIDGE: &str = "https://cli.cloud.tencent.com";

impl Store {
    pub(super) fn auth(&self, deed: &Auth) -> Result<Value> {
        if matches!(deed, Auth::Begin) {
            return self.begin();
        }
        let mut state = self.read()?;
        match deed {
            Auth::Complete { response } => self.complete(&mut state, response),
            Auth::Refresh => self.refresh(&mut state),
            Auth::Renew => self.renew(&mut state),
            Auth::Status => Ok(state.receipt()),
            Auth::Logout => {
                self.save(&State::fresh())?;
                Ok(
                    json!({"localLogout": true, "remoteRevocation": "not performed; existing provider grants and temporary credentials may remain valid"}),
                )
            }
            Auth::Begin => unreachable!(),
        }
    }

    fn begin(&self) -> Result<Value> {
        let exists = self
            .path
            .try_exists()
            .map_err(|_| anyhow::anyhow!("Tencent state presence could not be checked"))?;
        if exists {
            let earlier = self.read()?;
            if earlier.oauth.is_some() || earlier.credential.is_some() {
                bail!(
                    "Tencent authorization already exists; perform explicit local logout before beginning a new session"
                );
            }
            if let Some(intent) = &earlier.intent
                && !intent.consumed
                && intent.deadline > now()
            {
                return challenge(intent);
            }
        }
        let mut random = [0u8; 32];
        getrandom::fill(&mut random)
            .map_err(|_| anyhow::anyhow!("Tencent authorization entropy unavailable"))?;
        let mut state = State::fresh();
        state.intent = Some(Intent {
            state: sign::hash(&random),
            deadline: now() + 600,
            consumed: false,
        });
        if exists {
            self.save(&state)?;
        } else {
            self.create(&state)?;
        }
        challenge(state.intent.as_ref().expect("new intent is held"))
    }

    fn complete(&self, state: &mut State, path: &std::path::Path) -> Result<Value> {
        let intent = state
            .intent
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tencent authorization was not begun"))?;
        if intent.consumed || intent.deadline <= now() {
            bail!(
                "Tencent authorization expired or consumed; begin a new authorization at a new private state path"
            );
        }
        let bytes = secret::read(path)?;
        let decoded = Zeroizing::new(
            STANDARD
                .decode(bytes.trim_ascii())
                .map_err(|_| anyhow::anyhow!("Tencent private authorization response refused"))?,
        );
        let token = api::Body(
            serde_json::from_slice(&decoded)
                .map_err(|_| anyhow::anyhow!("Tencent private authorization response refused"))?,
        );
        if field(&token.0, "state")? != intent.state || field(&token.0, "site")? != "cn" {
            bail!("Tencent authorization state or site refused");
        }
        let oauth = Grant {
            subject: field(&token.0, "openId")?.into(),
            access: field(&token.0, "accessToken")?.into(),
            refresh: field(&token.0, "refreshToken")?.into(),
            expires: expiration(&token.0)?,
            site: "cn".into(),
        };
        state.oauth = Some(oauth);
        state.credential = None;
        state.identity = None;
        state.intent.as_mut().expect("intent is held").consumed = true;
        self.save(state)?;
        std::fs::remove_file(path).map_err(|_| anyhow::anyhow!("Tencent authorization was saved privately but its input could not be consumed; remove the private response file before explicit renewal"))?;
        self.renew(state)
    }

    fn refresh(&self, state: &mut State) -> Result<Value> {
        let oauth = state
            .oauth
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tencent authorization is absent"))?;
        let request = api::Body(
            json!({"TraceId": trace()?, "RefreshToken": oauth.refresh, "OpenId": oauth.subject, "Site": oauth.site}),
        );
        let response = api::post(
            &api::agent(),
            &format!("{BRIDGE}/refresh_user_token"),
            &request.0,
        )?;
        if response.0.get("Error").is_some() {
            bail!("Tencent authorization refresh refused; existing private state is retained");
        }
        let token = field(&response.0, "AccessToken")?.to_owned();
        let expiration = expiration(&response.0)?;
        let oauth = state.oauth.as_mut().expect("authorization is held");
        oauth.access.zeroize();
        oauth.access = token;
        oauth.expires = expiration;
        self.save(state)?;
        Ok(state.receipt())
    }

    fn renew(&self, state: &mut State) -> Result<Value> {
        let oauth = state
            .oauth
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tencent authorization is absent"))?;
        if oauth.expires <= now() {
            bail!(
                "Tencent authorization token expired; explicitly refresh before renewing cloud credentials"
            );
        }
        let request = api::Body(
            json!({"TraceId": trace()?, "AccessToken": oauth.access, "Site": oauth.site}),
        );
        let response = api::post(
            &api::agent(),
            &format!("{BRIDGE}/get_temp_cred"),
            &request.0,
        )?;
        if response.0.get("Error").is_some() {
            bail!("Tencent temporary credential exchange refused; no raw response is emitted");
        }
        let credential = Credential {
            id: field(&response.0, "SecretId")?.into(),
            key: field(&response.0, "SecretKey")?.into(),
            token: Some(field(&response.0, "Token")?.into()),
            expires: Some(expiration(&response.0)?),
        };
        state.credential = Some(credential);
        state.identity = None;
        self.save(state)?;
        Ok(state.receipt())
    }
}

fn expiration(value: &Value) -> Result<u64> {
    let end = value
        .get("expiresAt")
        .or_else(|| value.get("ExpiresAt"))
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("Tencent expiration refused"))?;
    if end <= now() {
        bail!("Tencent returned expired authorization data");
    }
    Ok(end)
}

fn trace() -> Result<String> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random)
        .map_err(|_| anyhow::anyhow!("Tencent request entropy unavailable"))?;
    let hash = sign::hash(&random);
    Ok(format!(
        "{}-{}-4{}-a{}-{}",
        &hash[..8],
        &hash[8..12],
        &hash[13..16],
        &hash[17..20],
        &hash[20..32]
    ))
}

fn challenge(intent: &Intent) -> Result<Value> {
    let mut url = url::Url::parse("https://cloud.tencent.com/open/authorize")
        .expect("static authorization URL");
    let redirect = "https://cli.cloud.tencent.com/oauth?browser=no&lang=zh-CN&site=cn";
    url.query_pairs_mut()
        .append_pair("scope", "login")
        .append_pair("app_id", "100038427476")
        .append_pair("redirect_url", redirect)
        .append_pair("state", &intent.state);
    Ok(
        json!({"authorizationUrl": url.as_str(), "expiresAt": intent.deadline, "application": "Tencent Cloud CLI official authorization application; native protocol compatibility is being qualified", "delivery": "Save the complete browser return privately in a local file; it contains bearer tokens and must not be pasted into chat. Complete consumes --response-file after durable private custody. Mainland site only."}),
    )
}
