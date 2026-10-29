pub(super) mod model;
pub(super) mod secret;

use self::model::{Credential, State, field, stamp};
use self::secret::Store;
use super::{Class, Failure, api};
use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct Identity {
    pub account: String,
    pub principal: String,
    pub kind: String,
    pub stamp: String,
    pub observed: u64,
    pub request: String,
}

pub(super) fn verify(
    store: &Store,
    state: &mut State,
    expected: Option<&str>,
    agent: crate::http::wire::Client,
) -> Result<Value> {
    let body = api::Client::using(state.credential()?, agent).send(
        "sts",
        "GetCallerIdentity",
        &json!({}),
    )?;
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

pub(super) fn authorize(state: &State, expected: Option<&str>) -> Result<()> {
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

pub(super) fn bound(
    state: &State,
    account: &str,
    credential: &Credential,
) -> Result<Identity, Failure> {
    authorize(state, Some(account)).map_err(|_| binding())?;
    let held = state.identity.as_ref().ok_or_else(binding)?;
    Ok(Identity {
        account: account.to_owned(),
        principal: model::field(held, "PrincipalId")
            .map_err(|_| binding())?
            .into(),
        kind: model::field(held, "Type").map_err(|_| binding())?.into(),
        stamp: model::stamp(credential),
        observed: held["verifiedAt"].as_u64().ok_or_else(binding)?,
        request: model::field(held, "RequestId")
            .map_err(|_| binding())?
            .into(),
    })
}

fn binding() -> Failure {
    Failure::new(
        Class::Binding,
        "Verify Tencent caller identity against the explicit owning account and current credentials before compute reads",
    )
}

#[cfg(test)]
mod tests {
    use super::{Credential, State, bound, stamp};
    use serde_json::json;

    #[test]
    fn legacy() {
        let credential = Credential {
            id: "fixture-id".into(),
            key: "PRIVATE_KEY_FIXTURE".into(),
            token: None,
            expires: None,
        };
        let value = json!({
            "version":1,
            "credential":{"id":credential.id,"key":credential.key},
            "identity":{"AccountId":"123","PrincipalId":"456","Type":"CAMUser","RequestId":"fixture-request","credentialStamp":stamp(&credential),"verifiedAt":1},
        });
        let mut state: State = serde_json::from_value(value).unwrap();
        let identity = bound(&state, "123", state.credential().unwrap()).unwrap();
        assert_eq!(identity.account, "123");
        assert_eq!(identity.principal, "456");
        assert_eq!(identity.kind, "CAMUser");
        assert_eq!(identity.observed, 1);
        assert_eq!(identity.request, "fixture-request");
        assert!(
            !serde_json::to_string(&identity)
                .unwrap()
                .contains("PRIVATE_KEY_FIXTURE")
        );
        state.identity.as_mut().unwrap()["credentialStamp"] = json!("changed");
        assert!(bound(&state, "123", state.credential().unwrap()).is_err());
    }
}
