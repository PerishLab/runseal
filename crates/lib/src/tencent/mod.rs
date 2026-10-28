mod api;
mod auth;
mod cam;
mod compute;
mod deed;
mod dns;
mod model;
mod secret;
use api::sign;

pub use api::{Class, Failure};
pub use compute::{
    Bundle, Client, Data, Identity, Image, Instance, Page, Product, Query, Read, Region, Shape,
    Window,
};

use crate::tool::Reply;
use anyhow::{Result, bail};
use clap::Parser;
use deed::{Cam, Deed, Input, Key};
use model::{Credential, State, field, stamp};
use secret::Store;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    let input = parse(argv)?;
    execute(input, vars).map(|value| Reply::plain("tencent", value))
}

pub fn run(argv: &[String], vars: &BTreeMap<String, String>) -> Result<()> {
    let parsed =
        Input::try_parse_from(std::iter::once("@tencent".to_string()).chain(argv.iter().cloned()));
    if let Err(error) = &parsed
        && matches!(
            error.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        )
    {
        print!("{error}");
        return Ok(());
    }
    let input = parsed.map_err(|_| anyhow::anyhow!("invalid @tencent command; use --help"))?;
    execute(input, vars)
        .map(|value| Reply::plain("tencent", value))?
        .emit(argv)
}

fn parse(argv: &[String]) -> Result<Input> {
    Input::try_parse_from(std::iter::once("@tencent".to_string()).chain(argv.iter().cloned()))
        .map_err(|_| anyhow::anyhow!("invalid @tencent command; use --help"))
}

fn execute(input: Input, vars: &BTreeMap<String, String>) -> Result<Value> {
    let path = input.state.or_else(|| vars.get("TENCENTCLOUD_STATE_FILE").map(Into::into)).ok_or_else(|| anyhow::anyhow!("@tencent requires --state or profile TENCENTCLOUD_STATE_FILE pointing to an absolute private state file"))?;
    let store = Store::open(&path)?;
    if let Deed::Auth(deed) = &input.deed {
        return store.auth(deed);
    }
    if let Deed::Compute(query) = &input.deed {
        let account = input.account.as_deref()
            .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str))
            .ok_or_else(|| anyhow::anyhow!("Tencent compute reads require an explicit owning account; verify identity first"))?;
        let reader = compute::Client::from(store, account)?;
        return serde_json::to_value(reader.read(query)?).map_err(Into::into);
    }
    let mut state = store.read()?;
    let account = input
        .account
        .as_deref()
        .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str));
    if matches!(input.deed, Deed::Identity) {
        return identity(&store, &mut state, account);
    }
    let (service, request) = match &input.deed {
        Deed::Cam(deed) => ("cam", cam::request(deed)?),
        Deed::Dns(deed) => ("dnspod", dns::request(deed)?),
        _ => unreachable!(),
    };
    if request.2 {
        authorize(&state, account)?;
    }
    if let Deed::Cam(Cam::Key(Key::Create { user, output })) = &input.deed {
        if Some(user.to_string()).as_deref() == account {
            bail!("Tencent root key issuance is outside this delegated-identity operation");
        }
        let reserved = secret::Reserved::open(output)?;
        let body = api::Client::new(state.credential()?).send(service, request.0, &request.1)?;
        return deliver(reserved, &body.0, *user);
    }
    let body = api::Client::new(state.credential()?).send(service, request.0, &request.1)?;
    Ok(json!({"action": request.0, "result": api::metadata(&body.0)}))
}

fn identity(store: &Store, state: &mut State, expected: Option<&str>) -> Result<Value> {
    let body =
        api::Client::new(state.credential()?).send("sts", "GetCallerIdentity", &json!({}))?;
    let account = field(&body.0, "AccountId")?;
    if !account.bytes().all(|byte| byte.is_ascii_digit())
        || expected.is_some_and(|value| value != account)
    {
        bail!("Tencent caller account does not match the explicit declaration");
    }
    let mut identity = api::metadata(&body.0);
    identity["credentialStamp"] = json!(stamp(state.credential()?));
    identity["verifiedAt"] = json!(model::now());
    state.identity = Some(identity);
    store.save(state)?;
    Ok(state.receipt())
}

fn authorize(state: &State, expected: Option<&str>) -> Result<()> {
    let expected = expected.ok_or_else(|| {
        anyhow::anyhow!("Tencent mutations require --account or profile TENCENTCLOUD_ACCOUNT_UIN")
    })?;
    let identity = state
        .identity
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("verify Tencent caller identity before a mutation"))?;
    if identity.get("AccountId").and_then(Value::as_str) != Some(expected)
        || identity.get("credentialStamp").and_then(Value::as_str)
            != Some(stamp(state.credential()?).as_str())
    {
        bail!(
            "Tencent caller binding refused; verify the current identity against the explicit account declaration"
        );
    }
    Ok(())
}

fn deliver(reserved: secret::Reserved, body: &Value, user: u64) -> Result<Value> {
    let key = body.get("AccessKey").ok_or_else(|| anyhow::anyhow!("Tencent key creation response lacked the issued key; inspect the explicit target user before recovery"))?;
    let mut issued = State::fresh();
    issued.credential = Some(Credential {
        id: field(key, "AccessKeyId")?.into(),
        key: field(key, "SecretAccessKey")?.into(),
        token: None,
        expires: None,
    });
    let receipt = json!({"action": "CreateAccessKey", "targetUin": user, "result": api::metadata(key), "requestId": body.get("RequestId").and_then(Value::as_str)});
    reserved.commit(&issued, receipt)
}

#[cfg(test)]
mod tests;
