use super::request::Plan;
use super::{Data, Receipt, Request, read};
use crate::alibaba::model::Credential;
use crate::alibaba::{Class, Error, Failure, Identity, Outcome, api};
use crate::http::wire;
use zeroize::Zeroizing;

pub(in crate::alibaba) fn send(
    credential: &Credential,
    caller: &Identity,
    request: &Request,
) -> Result<Receipt, Error> {
    let plan = request.plan().map_err(unsent)?;
    let query = query(&plan);
    if query.len() > 65536 {
        return Err(unsent(anyhow::anyhow!(
            "Alibaba RAM query allowance exceeded"
        )));
    }
    let date = api::sign::date(crate::alibaba::model::now()).map_err(unsent)?;
    let nonce = api::nonce().map_err(unsent)?;
    let digest = api::sign::hash(b"");
    let headers = [
        ("host", plan.host),
        ("x-acs-action", plan.action),
        ("x-acs-version", plan.version),
        ("x-acs-date", date.as_str()),
        ("x-acs-signature-nonce", nonce.as_str()),
        ("x-acs-content-sha256", digest.as_str()),
        ("x-acs-security-token", credential.token.as_str()),
    ];
    let authorization = Zeroizing::new(
        api::sign::authorization(
            (credential.id.as_str(), credential.key.as_str()),
            &headers,
            &query,
        )
        .map_err(unsent)?,
    );
    let mut headers = headers.to_vec();
    headers.push(("Authorization", authorization.as_str()));
    let url = format!("https://{}/?{query}", plan.host);
    let (status, bytes) = api::receive(
        &api::agent(),
        wire::Request {
            method: "POST",
            url: &url,
            headers: &headers,
            body: None,
        },
    )
    .map_err(|_| Error {
        fault: Failure::network("Alibaba RAM transport result was not observed"),
        outcome: Outcome::Unknown,
    })?;
    let (receipt, data, complete) = response(status, &bytes, request, &caller.account)?;
    Ok(Receipt {
        account: caller.account.clone(),
        principal: caller.principal.clone(),
        action: plan.action.into(),
        requested: request.target(),
        request: receipt,
        outcome: if request.mutation() {
            Outcome::Accepted
        } else {
            Outcome::Observed
        },
        data,
        complete,
    })
}

pub(super) fn response(
    status: u16,
    bytes: &[u8],
    request: &Request,
    account: &str,
) -> Result<(String, Data, bool), Error> {
    let body = api::decode(bytes, Class::Response).map_err(unknown)?;
    if status != 200 {
        let code = body.0.get("Code").and_then(serde_json::Value::as_str);
        let receipt = body.0.get("RequestId").and_then(serde_json::Value::as_str);
        return Err(Error::remote(Failure::acs(status, code, receipt)));
    }
    if body.0.get("Code").is_some() {
        return Err(unknown(anyhow::anyhow!(
            "Alibaba RAM success envelope contains an error code"
        )));
    }
    read::decode(&body.0, request, account).map_err(unknown)
}

pub(super) fn query(plan: &Plan) -> String {
    plan.fields
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            result.push(char::from(byte));
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

fn unsent(error: anyhow::Error) -> Error {
    Error {
        fault: Failure::capture(error, Class::Input),
        outcome: Outcome::Unsent,
    }
}

fn unknown(error: anyhow::Error) -> Error {
    Error {
        fault: Failure::capture(error, Class::Response),
        outcome: Outcome::Unknown,
    }
}
