use crate::github::{Cause, Error, Evidence, Outcome, Receipt};
use crate::http::wire::Payload;
use serde_json::Value;

pub(super) fn decode<T>(
    response: Payload,
    attempts: u8,
    graphql: bool,
    parse: impl Fn(Value, &Payload) -> Option<T>,
) -> Result<Receipt<T>, Error> {
    let mut evidence = evidence(&response, attempts);
    if !matches!(response.status, 200..=299) {
        return Err(status(&response, evidence));
    }
    let value: Value = serde_json::from_slice(&response.body).map_err(|_| Error {
        outcome: Outcome::Unknown,
        cause: Cause::Shape,
        evidence: evidence.clone(),
    })?;
    if graphql
        && value
            .get("errors")
            .is_some_and(|errors| !errors.as_array().is_some_and(Vec::is_empty))
    {
        evidence.partial = value.get("data").is_some_and(|data| !data.is_null());
        return Err(Error {
            outcome: Outcome::Unknown,
            cause: Cause::Graphql,
            evidence,
        });
    }
    let value = parse(value, &response).ok_or_else(|| Error {
        outcome: Outcome::Unknown,
        cause: Cause::Shape,
        evidence: evidence.clone(),
    })?;
    Ok(Receipt {
        outcome: Outcome::Observed,
        evidence,
        value,
    })
}

pub(super) fn evidence(response: &Payload, attempts: u8) -> Evidence {
    let header = |name: &str| response.headers.get(name).map(|text| text.as_str());
    Evidence {
        attempts,
        status: Some(response.status),
        request: identifier(header("x-github-request-id")),
        wait: header("retry-after").and_then(|text| text.parse().ok()),
        reset: header("x-ratelimit-reset").and_then(|text| text.parse().ok()),
        partial: false,
    }
}

pub(in crate::tool::github) fn identifier(value: Option<&str>) -> Option<String> {
    value
        .filter(|id| {
            !id.is_empty()
                && id.len() <= 96
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() || byte == b':')
        })
        .map(str::to_owned)
}

pub(super) fn status(response: &Payload, evidence: Evidence) -> Error {
    let remaining = response.headers.get("x-ratelimit-remaining");
    let exhausted = response.status == 403 && remaining.is_some_and(|left| left.as_str() == "0");
    let limited =
        response.status == 429 || response.headers.contains_key("retry-after") || exhausted;
    let cause = if limited { Cause::Limited } else { Cause::Http };
    let outcome = match response.status {
        400 | 401 | 403 | 404 | 405 | 409 | 410 | 422 | 429 => Outcome::Refused,
        _ => Outcome::Unknown,
    };
    Error {
        outcome,
        cause,
        evidence,
    }
}
