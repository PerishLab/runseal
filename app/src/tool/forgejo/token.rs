use anyhow::{Result, bail};
use serde_json::{Value, json};

use std::collections::BTreeMap;

use super::Seat;
use crate::{parse, tool::Reply};

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

impl Seat {
    pub(super) fn minted(&self, name: &str) -> Result<Value> {
        let Some(raw) = self.flag("scopes").filter(|held| !held.is_empty()) else {
            bail!("@forgejo token create requires --scopes");
        };
        let scopes: Vec<Value> = raw
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| Value::String(part.to_string()))
            .collect();
        if scopes.is_empty() {
            bail!("@forgejo token create requires at least one scope");
        }
        let body = json!({"name": name, "scopes": scopes});
        self.send("POST", &self.tokens()?, Some(&body))
    }

    pub(super) fn roster(&self) -> Result<Value> {
        self.send("GET", &self.tokens()?, None)
    }

    pub(super) fn revoked(&self, id: &str) -> Result<Value> {
        let stem = self.tokens()?;
        self.send("DELETE", &format!("{stem}/{id}"), None)
    }

    fn tokens(&self) -> Result<String> {
        let Some(who) = self.user.as_deref().filter(|held| !held.is_empty()) else {
            bail!("@forgejo token verbs require FORGEJO_USERNAME in the profile");
        };
        Ok(format!("{}/users/{who}/tokens", self.base))
    }
}

pub(super) fn basic(user: &str, password: &str) -> String {
    format!("Basic {}", armour(format!("{user}:{password}").as_bytes()))
}

fn armour(bytes: &[u8]) -> String {
    let mut held = String::new();
    for chunk in bytes.chunks(3) {
        let mut block = [0u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let packed = (u32::from(block[0]) << 16) | (u32::from(block[1]) << 8) | u32::from(block[2]);
        for slot in 0..4 {
            if slot <= chunk.len() {
                let index = (packed >> (18 - slot * 6)) & 0x3F;
                held.push(ALPHABET[index as usize] as char);
            } else {
                held.push('=');
            }
        }
    }
    held
}

pub(super) fn guarded(kind: &'static str, mut value: Value) -> Reply {
    let secret = value
        .as_object_mut()
        .and_then(|object| object.remove("sha1"))
        .and_then(|held| held.as_str().map(str::to_string));
    match secret {
        Some(secret) if !secret.is_empty() => Reply::guarded(kind, value, secret),
        _ => Reply::plain(kind, value),
    }
}

pub(super) fn password(line: &parse::Line, vars: &BTreeMap<String, String>) -> Result<String> {
    let Some(who) = vars.get("FORGEJO_USERNAME").filter(|held| !held.is_empty()) else {
        bail!("@forgejo token verbs require FORGEJO_USERNAME in the profile");
    };
    let secret = if let Some(file) = line.file.as_deref().filter(|held| !held.is_empty()) {
        super::load(file)?
    } else if let Some(held) = vars.get("FORGEJO_PASSWORD").filter(|held| !held.is_empty()) {
        held.clone()
    } else if let Some(file) = vars
        .get("FORGEJO_PASSWORD_FILE")
        .filter(|held| !held.is_empty())
    {
        super::load(file)?
    } else {
        bail!("@forgejo token verbs require FORGEJO_PASSWORD_FILE or FORGEJO_PASSWORD")
    };
    Ok(basic(who, &secret))
}
