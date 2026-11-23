use super::{Cause, Error, Evidence, Outcome, Receipt};
use crate::http::wire::{Client, Payload, Policy, Request};
use serde_json::Value;
use std::time::Duration;

pub(super) struct Wire {
    pub(super) origin: String,
    client: Client,
}

pub(super) enum Call<'a> {
    Rest(&'a str),
    Query(Value),
    Mutation(Value),
}

impl Wire {
    pub(super) fn new(origin: String) -> Self {
        Self {
            origin,
            client: Client::new(Policy {
                redirects: 0,
                overall: Some(Duration::from_secs(8)),
                connect: Some(Duration::from_secs(3)),
                limit: 1024 * 1024,
            }),
        }
    }

    pub(super) fn send<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        parse: impl Fn(Value) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        let (method, path, body) = match &call {
            Call::Rest(path) => ("GET", *path, None),
            Call::Query(body) | Call::Mutation(body) => {
                ("POST", "/graphql", Some(body.to_string()))
            }
        };
        let url = format!("{}{path}", self.origin);
        let headers = [
            ("authorization", bearer),
            ("accept", "application/vnd.github+json"),
            ("content-type", "application/json"),
            ("user-agent", "perishlab-runseal"),
            ("x-github-api-version", "2022-11-28"),
        ];
        let limit = if matches!(call, Call::Mutation(_)) {
            1
        } else {
            2
        };
        let mut number = 1;
        loop {
            let answer = self
                .request(
                    Request {
                        method,
                        url: &url,
                        headers: &headers,
                        body: body.as_deref().map(str::as_bytes),
                    },
                    number,
                )
                .and_then(|payload| decode(payload, number, body.is_some(), &parse));
            match answer {
                Err(error) if number < limit && retry(&error) => {
                    std::thread::sleep(Duration::from_millis(120));
                    number += 1;
                }
                result => return result,
            }
        }
    }

    fn request(&self, request: Request<'_>, attempts: u8) -> Result<Payload, Error> {
        let evidence = Evidence {
            attempts,
            ..Evidence::default()
        };
        let reply = self.client.send(request).map_err(|_| Error {
            outcome: Outcome::Unknown,
            cause: Cause::Network,
            evidence: evidence.clone(),
        })?;
        let status = Some(reply.status);
        reply.collect().map_err(|_| Error {
            outcome: Outcome::Unknown,
            cause: Cause::Read,
            evidence: Evidence { status, ..evidence },
        })
    }
}

fn retry(error: &Error) -> bool {
    if error.cause == Cause::Limited || error.evidence.wait.is_some() {
        return false;
    }
    match error.evidence.status {
        Some(502..=504) => error.cause == Cause::Http,
        None => error.cause == Cause::Network,
        _ => false,
    }
}

fn decode<T>(
    response: Payload,
    attempts: u8,
    graphql: bool,
    parse: impl Fn(Value) -> Option<T>,
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
    let value = parse(value).ok_or_else(|| Error {
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

fn evidence(response: &Payload, attempts: u8) -> Evidence {
    let header = |name: &str| response.headers.get(name).map(|text| text.as_str());
    Evidence {
        attempts,
        status: Some(response.status),
        request: header("x-github-request-id")
            .filter(|id| {
                !id.is_empty()
                    && id.len() <= 96
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() || byte == b':')
            })
            .map(str::to_owned),
        wait: header("retry-after").and_then(|text| text.parse().ok()),
        reset: header("x-ratelimit-reset").and_then(|text| text.parse().ok()),
        partial: false,
    }
}

fn status(response: &Payload, evidence: Evidence) -> Error {
    let remaining = response.headers.get("x-ratelimit-remaining");
    let exhausted = response.status == 403 && remaining.is_some_and(|left| left.as_str() == "0");
    let limited =
        response.status == 429 || response.headers.contains_key("retry-after") || exhausted;
    let cause = if limited { Cause::Limited } else { Cause::Http };
    let outcome = match response.status {
        400 | 401 | 403 | 404 | 409 | 410 | 422 | 429 => Outcome::Refused,
        _ => Outcome::Unknown,
    };
    Error {
        outcome,
        cause,
        evidence,
    }
}
