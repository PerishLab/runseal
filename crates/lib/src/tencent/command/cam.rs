use super::super::secret;
use super::{Cam, Key, Policy, User, dns::positive};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(in crate::tencent) fn request(deed: &Cam) -> Result<(&'static str, Value, bool)> {
    let found = match deed {
        Cam::User(User::Get(name)) => ("GetUser", json!({"Name": name.name}), false),
        Cam::User(User::Create(name)) => (
            "AddUser",
            json!({"Name": name.name, "ConsoleLogin": 0, "UseApi": 0}),
            true,
        ),
        Cam::Policy(Policy::List { page }) => (
            "ListPolicies",
            json!({"Scope": "Local", "Rp": 100, "Page": positive(u64::from(*page))?}),
            false,
        ),
        Cam::Policy(Policy::Get { id }) => {
            ("GetPolicy", json!({"PolicyId": positive(*id)?}), false)
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
            (
                "CreatePolicy",
                json!({"PolicyName": name, "PolicyDocument": encoded}),
                true,
            )
        }
        Cam::Policy(Policy::Attach(binding)) => (
            "AttachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "AttachUin": positive(binding.user)?}),
            true,
        ),
        Cam::Policy(Policy::Detach(binding)) => (
            "DetachUserPolicy",
            json!({"PolicyId": positive(binding.id)?, "DetachUin": positive(binding.user)?}),
            true,
        ),
        Cam::Key(Key::List { user }) => (
            "ListAccessKeys",
            json!({"TargetUin": positive(*user)?}),
            false,
        ),
        Cam::Key(Key::Create { user, .. }) => (
            "CreateAccessKey",
            json!({"TargetUin": positive(*user)?, "Description": "runseal scoped credential delivery"}),
            true,
        ),
        Cam::Key(Key::Disable(handle)) => (
            "UpdateAccessKey",
            json!({"TargetUin": positive(handle.user)?, "AccessKeyId": handle.id, "Status": "Inactive"}),
            true,
        ),
        Cam::Key(Key::Delete(handle)) => (
            "DeleteAccessKey",
            json!({"TargetUin": positive(handle.user)?, "AccessKeyId": handle.id}),
            true,
        ),
    };
    for key in ["Name", "PolicyName", "AccessKeyId"] {
        if let Some(value) = found.1.get(key).and_then(Value::as_str) {
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
