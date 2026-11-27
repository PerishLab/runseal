mod observe;
mod response;
mod transport;

pub use observe::{Observation, Sink, observe};
pub(in crate::tool::github) use response::identifier;
use response::{Mode, decode, evidence, retry, status};

use super::{Cause, Error, Outcome, Receipt};
use crate::http::wire::{Payload, Request};
use serde_json::Value;
use std::time::{Duration, Instant};
pub(super) use transport::LIMIT;
use transport::Transport;
use zeroize::Zeroizing;

pub(super) struct Wire {
    pub(super) origin: String,
    transport: Transport,
    sink: Option<Sink>,
}

pub(super) enum Call<'a> {
    Rest(&'a str),
    Issue(&'a str),
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
            transport: Transport::new(seconds),
            sink: None,
        }
    }

    pub(super) fn timeout(&mut self, seconds: u64) {
        self.transport.timeout(seconds);
    }

    pub(super) fn roots(&mut self, pem: &[u8]) -> Result<(), Error> {
        self.transport.roots(pem)
    }

    pub(super) fn observer(&mut self, sink: Sink) {
        self.sink = Some(sink);
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
        self.transact(bearer, call, false, parse)
    }

    pub(super) fn submit<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        parse: impl Fn(Value, &Payload) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        self.transact(bearer, call, true, parse)
    }

    fn transact<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        accepted: bool,
        parse: impl Fn(Value, &Payload) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        let started = Instant::now();
        let kind = observe::operation(&call);
        let result = self.execute(bearer, call, accepted, parse);
        observe::capture(self.sink, started, kind, &result);
        result
    }

    fn execute<T>(
        &self,
        bearer: &str,
        call: Call<'_>,
        accepted: bool,
        parse: impl Fn(Value, &Payload) -> Option<T>,
    ) -> Result<Receipt<T>, Error> {
        let (method, path, body) = match &call {
            Call::Rest(path)
            | Call::Issue(path)
            | Call::Source(path)
            | Call::Recent(path, None) => ("GET", *path, None),
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
        let mode = Mode::select(&call, accepted);
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
        let bytes = match call {
            Call::Source(_) => 4 * 1024 * 1024,
            Call::Survey(_) => 8 * 1024 * 1024,
            _ => 1024 * 1024,
        };
        let mut number = 1;
        loop {
            let answer = self
                .transport
                .request(
                    Request {
                        method,
                        url: &url,
                        headers: &headers,
                        body: body.as_deref().map(str::as_bytes),
                    },
                    number,
                    bytes,
                )
                .and_then(|payload| decode(payload, number, mode, &parse));
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
        let started = Instant::now();
        let kind = ("redirect", observe::route("redirect", path));
        let result = self.locate(bearer, path);
        observe::capture(self.sink, started, kind, &result);
        result
    }

    fn locate(&self, bearer: &str, path: &str) -> Result<Receipt<Zeroizing<String>>, Error> {
        let headers = headers(bearer, "2022-11-28");
        let url = format!("{}{path}", self.origin);
        let payload = self.transport.request(
            Request {
                method: "GET",
                url: &url,
                headers: &headers,
                body: None,
            },
            1,
            1024 * 1024,
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

    pub(super) fn accept(
        &self,
        bearer: &str,
        path: &str,
        body: Value,
    ) -> Result<Receipt<()>, Error> {
        let started = Instant::now();
        let kind = ("rest", observe::route("post", path));
        let result = self.post(bearer, path, &body.to_string());
        observe::capture(self.sink, started, kind, &result);
        result
    }

    fn post(&self, bearer: &str, path: &str, body: &str) -> Result<Receipt<()>, Error> {
        let headers = headers(bearer, "2022-11-28");
        let url = format!("{}{path}", self.origin);
        let payload = self.transport.request(
            Request {
                method: "POST",
                url: &url,
                headers: &headers,
                body: Some(body.as_bytes()),
            },
            1,
            1024 * 1024,
        )?;
        let evidence = evidence(&payload, 1);
        if payload.status == 204 && payload.body.is_empty() {
            return Ok(Receipt {
                outcome: Outcome::Accepted,
                evidence,
                value: (),
            });
        }
        if matches!(payload.status, 200..=299) {
            return Err(Error {
                outcome: Outcome::Unknown,
                cause: Cause::Shape,
                evidence,
            });
        }
        Err(status(&payload, evidence))
    }
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

#[cfg(test)]
mod tests;
