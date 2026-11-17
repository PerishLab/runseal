use super::super::{
    api,
    model::{self, Credential, Pending, Retained, State},
    secret::Store,
};
use super::{available, result, save, unknown};
use crate::http::wire;
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use zeroize::Zeroizing;

pub(super) fn renew(store: &Store) -> Result<Value> {
    let mut state = store.read()?;
    available(&state)?;
    let held = state
        .federation
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba federation binding missing"))?;
    let token = held
        .token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Acquire an Ensign token explicitly before STS renewal"))?;
    if token.expires <= model::now() {
        bail!("Ensign token expired; acquire a new token explicitly before STS renewal");
    }
    let query = query(&held.binding, &token.value);
    state.credential = None;
    state
        .federation
        .as_mut()
        .expect("checked federation")
        .identity = None;
    state.pending = Some(Pending {
        stage: model::ROLE.into(),
        started: model::now(),
    });
    save(store, &state, false)?;
    let (status, body) = send(&api::agent(), &query).map_err(|_| {
        unknown("Alibaba STS role dispatch result was not observed; saved intent forbids replay")
    })?;
    state.retained = Some(Retained {
        stage: model::ROLE.into(),
        status,
        body: STANDARD.encode(&body),
    });
    save(store, &state, true)?;
    result::settle(store, &mut state)
}

pub(super) fn query(binding: &super::Binding, token: &str) -> Zeroizing<String> {
    let duration = binding.duration.to_string();
    let mut fields = vec![
        ("Action", "AssumeRoleWithOIDC"),
        ("Version", api::VERSION),
        ("OIDCProviderArn", binding.provider.as_str()),
        ("RoleArn", binding.role.as_str()),
        ("RoleSessionName", binding.session.as_str()),
        ("DurationSeconds", duration.as_str()),
        ("OIDCToken", token),
        ("Format", "JSON"),
    ];
    fields.sort_by(|a, b| a.0.cmp(b.0));
    Zeroizing::new(
        fields
            .into_iter()
            .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
            .collect::<Vec<_>>()
            .join("&"),
    )
}

fn send(agent: &wire::Client, query: &str) -> Result<(u16, Zeroizing<Vec<u8>>)> {
    let url = Zeroizing::new(format!("{}/?{}", api::STS, query));
    let headers = [
        ("x-acs-action", "AssumeRoleWithOIDC"),
        ("x-acs-version", api::VERSION),
    ];
    api::receive(
        agent,
        wire::Request {
            method: "POST",
            url: &url,
            headers: &headers,
            body: None,
        },
    )
    .map_err(Into::into)
}

pub(super) fn apply(state: &mut State, value: &Value, pending: &Pending) -> Result<()> {
    if value.get("Code").is_some() {
        bail!("Alibaba STS success response contains a provider error");
    }
    let held = state
        .federation
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba role binding missing"))?;
    let info = &value["OIDCTokenInfo"];
    let binding = &held.binding;
    let exact = model::field(info, "Issuer")? == binding.issuer
        && model::field(info, "Subject")? == binding.subject
        && model::field(info, "ClientIds")? == binding.audience
        && model::field(info, "VerificationInfo")? == "Success";
    if !exact {
        bail!(
            "Alibaba STS issuer, subject, client or verification differs from declared federation"
        );
    }
    let token = held
        .token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Alibaba exchange has no retained upstream token"))?;
    if timestamp(model::field(info, "ExpirationTime")?)? != token.expires
        || timestamp(model::field(info, "IssuanceTime")?)? != token.issued
    {
        bail!("Alibaba STS OIDC times differ from the verified retained token");
    }
    let assumed = &value["AssumedRoleUser"];
    model::field(assumed, "AssumedRoleId")?;
    model::field(value, "RequestId")?;
    if model::field(assumed, "Arn")? != binding.arn() {
        bail!("Alibaba STS assumed session differs from declared role and session");
    }
    let received = &value["Credentials"];
    let expires = time::OffsetDateTime::parse(
        model::field(received, "Expiration")?,
        &time::format_description::well_known::Rfc3339,
    )?
    .unix_timestamp();
    let expires = u64::try_from(expires)?;
    if expires <= model::now()
        || expires
            > pending
                .started
                .saturating_add(binding.duration)
                .saturating_add(60)
    {
        bail!("Alibaba role credential expiry exceeds the declared request or has expired");
    }
    let credential = Credential {
        id: model::field(received, "AccessKeyId")?.into(),
        key: model::field(received, "AccessKeySecret")?.into(),
        token: model::field(received, "SecurityToken")?.into(),
        expires,
        observed: pending.started,
    };
    state.credential = Some(credential);
    state
        .federation
        .as_mut()
        .expect("checked federation")
        .identity = None;
    Ok(())
}

fn encode(value: &str) -> String {
    let mut text = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            text.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(text, "%{byte:02X}").expect("string writing");
        }
    }
    text
}

fn timestamp(value: &str) -> Result<u64> {
    let time = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)?;
    u64::try_from(time.unix_timestamp()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::query;
    use crate::alibaba::Binding;
    use serde_json::json;

    #[test]
    fn wire() {
        let binding: Binding = serde_json::from_value(json!({
            "source":"https://identity.example/api", "issuer":"https://identity.example/api",
            "subject":"external-subject", "audience":"cloud-client", "account":"1234567890",
            "provider":"acs:ram::1234567890:oidc-provider/identity",
            "role":"acs:ram::1234567890:role/operator", "id":"2345678901",
            "session":"fixture@session", "duration":900
        }))
        .unwrap();
        binding.validate().unwrap();
        let query = query(&binding, "private+/=?& fixture");
        assert!(query.contains("OIDCToken=private%2B%2F%3D%3F%26%20fixture"));
        let fields = url::form_urlencoded::parse(query.as_bytes())
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.len(), 8);
        assert_eq!(fields["Action"], "AssumeRoleWithOIDC");
        assert_eq!(fields["Version"], "2015-04-01");
        assert_eq!(fields["RoleArn"], binding.role);
        assert_eq!(fields["OIDCProviderArn"], binding.provider);
        assert_eq!(fields["RoleSessionName"], "fixture@session");
        assert_eq!(fields["DurationSeconds"], "900");
        assert_eq!(fields["OIDCToken"], "private+/=?& fixture");
        for forbidden in ["AccessKeyId", "Signature", "SecurityToken", "Policy"] {
            assert!(!fields.contains_key(forbidden));
        }
    }
}
