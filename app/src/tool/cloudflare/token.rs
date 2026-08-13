use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{
    Seat,
    api::Client,
    deed::{Action, Owner, Token},
};
use crate::{parse, tool::Reply};

struct Turn {
    page: usize,
    count: usize,
    total: usize,
}

pub(super) fn act(seat: &Seat, client: &Client, token: &Token) -> Result<Reply> {
    let stem = scope(seat, token.owner)?;
    let page = match &token.kind {
        Action::List => return listed(seat, client, &stem),
        Action::Show(id) => client.send("GET", &format!("{stem}/{}", segment(id)), None)?,
        Action::Create => {
            let body = seat.body()?;
            client.send("POST", &stem, Some(&body))?
        }
        Action::Edit(id) => {
            let body = seat.body()?;
            client.send("PUT", &format!("{stem}/{}", segment(id)), Some(&body))?
        }
        Action::Roll(id) => {
            let mut page = client.send(
                "PUT",
                &format!("{stem}/{}/value", segment(id)),
                Some(&json!({})),
            )?;
            if let Some(object) = page.result.as_object_mut() {
                object
                    .entry("id")
                    .or_insert_with(|| Value::String(id.clone()));
            }
            page
        }
        Action::Drop(id) => client.send(
            "DELETE",
            &format!("{stem}/{}", segment(id)),
            Some(&json!({})),
        )?,
        Action::Verify => client.send("GET", &format!("{stem}/verify"), None)?,
        Action::Permissions => {
            let route = filters(&seat.line, &format!("{stem}/permission_groups"));
            client.send("GET", &route, None)?
        }
    };
    reply(token.label(), page.result)
}

fn scope(seat: &Seat, owner: Owner) -> Result<String> {
    match owner {
        Owner::Account => Ok(format!("{}/tokens", seat.account()?)),
        Owner::User => Ok("/user/tokens".into()),
    }
}

fn listed(seat: &Seat, client: &Client, stem: &str) -> Result<Reply> {
    let mut rows = Vec::new();
    let mut page = 1usize;
    loop {
        let route = listing(&seat.line, stem, page);
        let held = client.send("GET", &route, None)?;
        let Some(found) = held.result.as_array() else {
            bail!("cloudflare token list did not return an array");
        };
        let count = found.len();
        rows.extend(found.iter().cloned());
        let turn = Turn {
            page,
            count,
            total: rows.len(),
        };
        if done(&seat.line, &held.info, &turn) {
            break;
        }
        page += 1;
    }
    if let Some(limit) = seat.line.limit {
        rows.truncate(limit);
    }
    Ok(Reply::plain("tokens", Value::Array(rows)))
}

fn reply(kind: &'static str, mut value: Value) -> Result<Reply> {
    let secret = value
        .as_object_mut()
        .and_then(|object| object.remove("value"))
        .and_then(|value| value.as_str().map(str::to_string));
    match secret {
        Some(secret) if !secret.is_empty() => Ok(Reply::guarded(kind, value, secret)),
        Some(_) => bail!("cloudflare returned an empty token secret"),
        None => Ok(Reply::plain(kind, value)),
    }
}

fn done(line: &parse::Line, info: &Value, turn: &Turn) -> bool {
    if line.limit.is_some_and(|limit| turn.total >= limit) {
        return true;
    }
    info.get("total_pages")
        .and_then(Value::as_u64)
        .map(|pages| turn.page as u64 >= pages)
        .unwrap_or(turn.count < 50)
}

fn listing(line: &parse::Line, stem: &str, page: usize) -> String {
    let mut route = format!("{stem}?page={page}&per_page=50");
    for name in ["direction", "include-expired"] {
        if let Some(value) = line.flag(name) {
            route.push('&');
            route.push_str(&name.replace('-', "_"));
            route.push('=');
            route.push_str(&query(value));
        }
    }
    route
}

fn filters(line: &parse::Line, stem: &str) -> String {
    let mut parts = Vec::new();
    for name in ["name", "scope"] {
        if let Some(value) = line.flag(name) {
            parts.push(format!("{name}={}", query(value)));
        }
    }
    if parts.is_empty() {
        stem.to_string()
    } else {
        format!("{stem}?{}", parts.join("&"))
    }
}

pub(super) fn segment(value: &str) -> String {
    encode(value)
}

fn query(value: &str) -> String {
    encode(value)
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
