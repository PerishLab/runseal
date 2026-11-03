mod model;
mod record;
mod retirement;
mod types;

pub use retirement::Request;
pub use types::{Data, Entity, Page, Receipt, Relation};

use super::super::secret;
use super::{Cam, Key, Policy, User, dns::positive};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(in crate::tencent) fn request(deed: &Cam) -> Result<super::Request> {
    let found = match deed {
        Cam::Policy(Policy::Entities { .. } | Policy::Delete { .. }) => {
            bail!("CAM policy operation requires its verified native client")
        }
        Cam::User(User::Get(name)) => super::Request::read("GetUser", json!({"Name": name.name})),
        Cam::User(User::Create(name)) => super::Request::write(
            "AddUser",
            json!({"Name": name.name, "ConsoleLogin": 0, "UseApi": 0}),
        ),
        Cam::Policy(Policy::List { page }) => super::Request::read(
            "ListPolicies",
            json!({"Scope": "Local", "Rp": 100, "Page": positive(u64::from(*page))?}),
        ),
        Cam::Policy(Policy::Get { id }) => {
            super::Request::read("GetPolicy", json!({"PolicyId": positive(*id)?}))
        }
        Cam::Policy(Policy::Create { name, document }) => {
            let data = secret::read(document)?;
            let document: Value = serde_json::from_slice(&data)
                .map_err(|_| anyhow::anyhow!("Tencent policy document refused"))?;
            if document.get("version").and_then(Value::as_str) != Some("2.0")
                || document
                    .get("statement")
                    .and_then(Value::as_array)
                    .is_none_or(|items| items.is_empty())
            {
                bail!("Tencent policy document requires version 2.0 and nonempty statements");
            }
            let encoded = serde_json::to_string(&document)
                .map_err(|_| anyhow::anyhow!("Tencent policy document refused"))?;
            super::Request::write(
                "CreatePolicy",
                json!({"PolicyName": name, "PolicyDocument": encoded}),
            )
        }
        Cam::Policy(Policy::Attach(binding)) => super::Request::write(
            "AttachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "AttachUin": positive(binding.user)?}),
        ),
        Cam::Policy(Policy::Detach(binding)) => super::Request::write(
            "DetachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "DetachUin": positive(binding.user)?}),
        ),
        Cam::Key(Key::List { user }) => {
            super::Request::read("ListAccessKeys", json!({"TargetUin": positive(*user)?}))
        }
        Cam::Key(Key::Create { user, .. }) => super::Request::write(
            "CreateAccessKey",
            json!({"TargetUin": positive(*user)?, "Description": "runseal scoped credential delivery"}),
        ),
        Cam::Key(Key::Disable(handle)) => super::Request::write(
            "UpdateAccessKey",
            json!({"TargetUin": positive(handle.user)?, "AccessKeyId": handle.id, "Status": "Inactive"}),
        ),
        Cam::Key(Key::Delete(handle)) => super::Request::write(
            "DeleteAccessKey",
            json!({"TargetUin": positive(handle.user)?, "AccessKeyId": handle.id}),
        ),
    };
    for key in ["Name", "PolicyName", "AccessKeyId"] {
        if let Some(value) = found.body.get(key).and_then(Value::as_str) {
            let width = (1..=128).contains(&value.len());
            let alphabet = value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'@')
            });
            if !(width && alphabet) {
                bail!("Tencent resource name refused");
            }
        }
    }
    Ok(found)
}

pub(in crate::tencent) fn receipt(deed: &Cam, value: &Value) -> Result<Value> {
    use super::super::api::project;
    match deed {
        Cam::Policy(Policy::Entities { .. } | Policy::Delete { .. }) => {
            bail!("CAM policy receipt requires its finite native model")
        }
        Cam::User(_) => project::<model::User>(value),
        Cam::Policy(Policy::List { .. }) => project::<model::Policies>(value),
        Cam::Policy(Policy::Get { .. }) => project::<model::Document>(value),
        Cam::Policy(Policy::Create { .. }) => project::<model::Created>(value),
        Cam::Key(Key::List { .. }) => project::<model::Keys>(value),
        Cam::Key(Key::Create { .. }) => project::<model::Issued>(value),
        Cam::Policy(Policy::Attach(_) | Policy::Detach(_))
        | Cam::Key(Key::Disable(_) | Key::Delete(_)) => project::<model::Ack>(value),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod checks;

#[cfg(test)]
mod protocol;

pub(in crate::tencent) fn convert(policy: &Policy) -> Option<Request> {
    match policy {
        Policy::Entities { id, page } => Some(Request::Entities {
            policy: *id,
            page: *page,
        }),
        Policy::Delete { id } => Some(Request::Delete { policy: *id }),
        _ => None,
    }
}
