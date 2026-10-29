mod api;
mod client;
mod command;
mod compute;
pub mod dns;
mod identity;
pub use identity::Identity;
use identity::{model, secret};
pub mod network;

pub use api::{Class, Error, Failure, Outcome};
pub use client::Client;
pub use compute::{
    Bundle, Data, Image, Instance, Page, Product, Query, Read, Region, Shape, Window,
};

use crate::tool::Reply;
use anyhow::{Result, bail};
use clap::Parser;
use command::{Cam, Deed, Input, Key, Mode};
use model::{Credential, State, field};
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
    if let Deed::Network(request) = &input.deed {
        let account = input
            .account
            .as_deref()
            .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Tencent network requires an explicit owning account and verified caller"
                )
            })?;
        let client = Client::from(store, account)?;
        return serde_json::to_value(client.network(request)?).map_err(Into::into);
    }
    if let Deed::Dns(deed) = &input.deed {
        let account = input
            .account
            .as_deref()
            .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str))
            .ok_or_else(|| {
                anyhow::anyhow!("DNSPod requires an explicit owning account and verified caller")
            })?;
        let request = command::dns::request(deed)?;
        return serde_json::to_value(Client::from(store, account)?.dns(&request)?)
            .map_err(Into::into);
    }
    if let Deed::Compute(query) = &input.deed {
        let account = input.account.as_deref()
            .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str))
            .ok_or_else(|| anyhow::anyhow!("Tencent compute reads require an explicit owning account; verify identity first"))?;
        let reader = Client::from(store, account)?;
        return serde_json::to_value(reader.read(&query.clone().into())?).map_err(Into::into);
    }
    let mut state = store.read()?;
    let account = input
        .account
        .as_deref()
        .or_else(|| vars.get("TENCENTCLOUD_ACCOUNT_UIN").map(String::as_str));
    if matches!(input.deed, Deed::Identity) {
        return identity::verify(&store, &mut state, account, api::agent());
    }
    let (service, request) = match &input.deed {
        Deed::Cam(deed) => ("cam", command::cam::request(deed)?),
        _ => unreachable!(),
    };
    if request.mode == Mode::Write {
        identity::authorize(&state, account)?;
    }
    let reserved = if let Deed::Cam(Cam::Key(Key::Create { user, output })) = &input.deed {
        if Some(user.to_string()).as_deref() == account {
            bail!("Tencent root key issuance is outside this delegated-identity operation");
        }
        Some(secret::Reserved::open(output)?)
    } else {
        None
    };
    let body =
        api::Client::new(state.credential()?).send(service, request.action, &request.body)?;
    let result = match &input.deed {
        Deed::Cam(deed) => command::cam::receipt(deed, &body.0)?,
        _ => unreachable!(),
    };
    if let (Deed::Cam(Cam::Key(Key::Create { user, .. })), Some(reserved)) = (&input.deed, reserved)
    {
        return deliver(reserved, &body.0, *user, result);
    }
    Ok(json!({"action": request.action, "result": result}))
}

fn deliver(reserved: secret::Reserved, body: &Value, user: u64, result: Value) -> Result<Value> {
    let unavailable = || {
        let mut fault = Failure::new(
            Class::Response,
            "Tencent key issuance receipt lacked private delivery material; a mutation may have completed; inspect the explicit target user before recovery",
        );
        fault.request = result["RequestId"].as_str().map(str::to_owned);
        fault
    };
    let key = body.get("AccessKey").ok_or_else(unavailable)?;
    let mut issued = State::fresh();
    issued.credential = Some(Credential {
        id: field(key, "AccessKeyId").map_err(|_| unavailable())?.into(),
        key: field(key, "SecretAccessKey")
            .map_err(|_| unavailable())?
            .into(),
        token: None,
        expires: None,
    });
    let receipt = json!({"action": "CreateAccessKey", "targetUin": user, "result": result["AccessKey"], "requestId": body.get("RequestId").and_then(Value::as_str)});
    reserved.commit(&issued, receipt)
}
