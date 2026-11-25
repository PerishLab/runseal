use super::{Cause, Error, Evidence, Outcome, Receipt};
use crate::http::wire::{Client, Payload, Policy, Request};
use serde_json::Value;
use std::time::Duration;
use zeroize::Zeroizing;

pub(super) struct Wire {
    pub(super) origin: String,
    client: Client,
    source: Client,
    survey: Client,
}

pub(super) enum Call<'a> {
    Rest(&'a str),
    Recent(&'a str, Option<Value>),
    Source(&'a str),
    Put(&'a str, Value),
    Post(&'a str, Value),
    Patch(&'a str, Value),
    Dispatch(&'a str, Value),
    Query(Value),
    Raw(Value),
    Survey(Value),
    Mutation(Value),
}

impl Wire {
    pub(super) fn new(origin: String) -> Self {
        Self::bounded(origin, 8)
    }

    pub(super) fn bounded(origin: String, seconds: u64) -> Self {
        Self {
            origin,
            client: client(1024 * 1024, seconds),
            source: client(4 * 1024 * 1024, seconds),
            survey: client(8 * 1024 * 1024, seconds),
        }
    }

    pub(super) fn timeout(&mut self, seconds: u64) {
        self.client = client(1024 * 1024, seconds);
        self.source = client(4 * 1024 * 1024, seconds);
        self.survey = client(8 * 1024 * 1024, seconds);
    }

    pub(super) fn send<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        parse: impl Fn(Value) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        self.inspect(bearer, call, |value, _| parse(value))
    }

    pub(super) fn inspect<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        parse: impl Fn(Value, &Payload) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        let (method, path, body) = match &call {
            Call::Rest(path) | Call::Source(path) | Call::Recent(path, None) => {
                ("GET", *path, None)
            }
            Call::Dispatch(path, body)
            | Call::Post(path, body)
            | Call::Recent(path, Some(body)) => ("POST", *path, Some(body.to_string())),
            Call::Patch(path, body) => ("PATCH", *path, Some(body.to_string())),
            Call::Put(path, body) => ("PUT", *path, Some(body.to_string())),
            Call::Query(body) | Call::Raw(body) | Call::Survey(body) | Call::Mutation(body) => {
                ("POST", "/graphql", Some(body.to_string()))
            }
        };
        let url = format!("{}{path}", self.origin);
        let version = if matches!(call, Call::Dispatch(..) | Call::Recent(..)) {
            "2026-03-10"
        } else {
            "2022-11-28"
        };
        let headers = headers(bearer, version);
        let graphql = matches!(call, Call::Query(_) | Call::Survey(_) | Call::Mutation(_));
        let limit = if matches!(
            call,
            Call::Mutation(_)
                | Call::Put(..)
                | Call::Post(..)
                | Call::Patch(..)
                | Call::Dispatch(..)
                | Call::Recent(_, Some(_))
        ) {
            1
        } else {
            2
        };
        let http = match call {
            Call::Source(_) => &self.source,
            Call::Survey(_) => &self.survey,
            _ => &self.client,
        };
        let mut number = 1;
        loop {
            let answer = request(
                http,
                Request {
                    method,
                    url: &url,
                    headers: &headers,
                    body: body.as_deref().map(str::as_bytes),
                },
                number,
            )
            .and_then(|payload| decode(payload, number, graphql, &parse));
            match answer {
                Err(error) if number < limit && retry(&error) => {
                    std::thread::sleep(Duration::from_millis(120));
                    number += 1;
                }
                result => return result,
            }
        }
    }

    pub(super) fn redirect(
        &self,
        bearer: &str,
        path: &str,
    ) -> Result<Receipt<Zeroizing<String>>, Error> {
        let headers = headers(bearer, "2022-11-28");
        let url = format!("{}{path}", self.origin);
        let payload = request(
            &self.client,
            Request {
                method: "GET",
                url: &url,
                headers: &headers,
                body: None,
            },
            1,
        )?;
        let evidence = evidence(&payload, 1);
        if payload.status != 302 {
            return Err(status(&payload, evidence));
        }
        let location = payload.headers.get("location").ok_or_else(|| Error {
            outcome: Outcome::Unknown,
            cause: Cause::Shape,
            evidence: evidence.clone(),
        })?;
        Ok(Receipt {
            outcome: Outcome::Observed,
            evidence,
            value: location.clone(),
        })
    }
}

fn request(client: &Client, request: Request<'_>, attempts: u8) -> Result<Payload, Error> {
    let evidence = Evidence {
        attempts,
        ..Evidence::default()
    };
    let reply = client.send(request).map_err(|_| Error {
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

fn client(limit: u64, seconds: u64) -> Client {
    Client::new(Policy {
        redirects: 0,
        overall: Some(Duration::from_secs(seconds)),
        connect: Some(Duration::from_secs(seconds.min(3))),
        limit,
    })
}

fn headers<'a>(bearer: &'a str, version: &'static str) -> [(&'static str, &'a str); 5] {
    [
        ("authorization", bearer),
        ("accept", "application/vnd.github+json"),
        ("content-type", "application/json"),
        ("user-agent", "perishlab-runseal"),
        ("x-github-api-version", version),
    ]
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

fn evidence(response: &Payload, attempts: u8) -> Evidence {
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

pub(super) fn identifier(value: Option<&str>) -> Option<String> {
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

fn status(response: &Payload, evidence: Evidence) -> Error {
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
