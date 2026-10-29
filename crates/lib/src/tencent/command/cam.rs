use super::super::secret;
use super::{Cam, Key, Policy, Request, User, dns::positive};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(in crate::tencent) fn request(deed: &Cam) -> Result<Request> {
    let found = match deed {
        Cam::User(User::Get(name)) => Request::read("GetUser", json!({"Name": name.name})),
        Cam::User(User::Create(name)) => Request::write(
            "AddUser",
            json!({"Name": name.name, "ConsoleLogin": 0, "UseApi": 0}),
        ),
        Cam::Policy(Policy::List { page }) => Request::read(
            "ListPolicies",
            json!({"Scope": "Local", "Rp": 100, "Page": positive(u64::from(*page))?}),
        ),
        Cam::Policy(Policy::Get { id }) => {
            Request::read("GetPolicy", json!({"PolicyId": positive(*id)?}))
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
            Request::write(
                "CreatePolicy",
                json!({"PolicyName": name, "PolicyDocument": encoded}),
            )
        }
        Cam::Policy(Policy::Attach(binding)) => Request::write(
            "AttachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "AttachUin": positive(binding.user)?}),
        ),
        Cam::Policy(Policy::Detach(binding)) => Request::write(
            "DetachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "DetachUin": positive(binding.user)?}),
        ),
        Cam::Key(Key::List { user }) => {
            Request::read("ListAccessKeys", json!({"TargetUin": positive(*user)?}))
        }
        Cam::Key(Key::Create { user, .. }) => Request::write(
            "CreateAccessKey",
            json!({"TargetUin": positive(*user)?, "Description": "runseal scoped credential delivery"}),
        ),
        Cam::Key(Key::Disable(handle)) => Request::write(
            "UpdateAccessKey",
            json!({"TargetUin": positive(handle.user)?, "AccessKeyId": handle.id, "Status": "Inactive"}),
        ),
        Cam::Key(Key::Delete(handle)) => Request::write(
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

pub(in crate::tencent) fn receipt(deed: &Cam, value: &Value) -> Result<()> {
    let valid = match deed {
        Cam::User(_) => value
            .get("Uin")
            .is_some_and(|held| held.is_u64() || held.is_string()),
        Cam::Policy(Policy::List { .. }) => value
            .get("List")
            .is_some_and(|held| held.is_array() || held.is_null()),
        Cam::Policy(Policy::Get { .. }) => {
            value.get("PolicyDocument").is_some_and(Value::is_string)
        }
        Cam::Policy(Policy::Create { .. }) => value.get("PolicyId").is_some_and(Value::is_u64),
        Cam::Key(Key::List { .. }) => value
            .get("AccessKeys")
            .is_some_and(|held| held.is_array() || held.is_null()),
        Cam::Key(Key::Create { .. }) => value.get("AccessKey").is_some_and(Value::is_object),
        Cam::Policy(Policy::Attach(_) | Policy::Detach(_))
        | Cam::Key(Key::Disable(_) | Key::Delete(_)) => true,
    };
    super::receipt(valid)
}
