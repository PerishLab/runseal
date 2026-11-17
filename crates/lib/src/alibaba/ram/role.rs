use super::Slot;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub trust: Value,
    pub lifetime: u64,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Data {
    pub name: String,
    pub arn: String,
    pub id: String,
    pub trust: Slot<String>,
    pub lifetime: Slot<u64>,
    pub console: Slot<bool>,
    pub linked: Slot<bool>,
    pub deletion: Slot<Deletion>,
    pub description: Slot<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Deletion {
    pub id: Slot<String>,
    pub created: Slot<String>,
}

pub(super) fn decode(value: &Value, name: &str, account: &str) -> anyhow::Result<(Data, bool)> {
    use super::project::{Shape, arn, complete, slot, text};
    let observed = text(value, "RoleName")?;
    if observed != name {
        anyhow::bail!("Alibaba role response names a different target");
    }
    let resource = text(value, "Arn")?;
    arn(resource, account, "role", name)?;
    let id = text(value, "RoleId")?;
    if !id.bytes().all(|v| v.is_ascii_digit()) || id.parse::<u64>().ok().is_none_or(|v| v == 0) {
        anyhow::bail!("Alibaba RAM role ID refused");
    }
    let (deletion, nested) = deletion(value)?;
    let data = Data {
        name: observed.into(),
        arn: resource.into(),
        id: id.into(),
        trust: slot(value, "AssumeRolePolicyDocument")?,
        lifetime: slot(value, "MaxSessionDuration")?,
        console: slot(value, "AllowConsoleLogin")?,
        linked: slot(value, "IsServiceLinkedRole")?,
        deletion,
        description: slot(value, "Description")?,
    };
    let known = complete(
        value,
        &[
            ("RoleName", Shape::Text),
            ("Arn", Shape::Text),
            ("RoleId", Shape::Text),
            ("AssumeRolePolicyDocument", Shape::Text),
            ("MaxSessionDuration", Shape::Number),
            ("AllowConsoleLogin", Shape::Boolean),
            ("IsServiceLinkedRole", Shape::Boolean),
            ("Description", Shape::Text),
            ("CreateDate", Shape::Text),
            ("UpdateDate", Shape::Text),
            ("RolePrincipalName", Shape::Text),
            ("LatestDeletionTask", Shape::Object),
        ],
    );
    Ok((data, known && nested))
}

fn deletion(value: &Value) -> anyhow::Result<(Slot<Deletion>, bool)> {
    use super::project::{Shape, complete, slot};
    match value.get("LatestDeletionTask") {
        None => Ok((Slot::Missing, true)),
        Some(Value::Null) => Ok((Slot::Null, true)),
        Some(row) if row.is_object() => {
            let data = Deletion {
                id: slot(row, "DeletionTaskId")?,
                created: slot(row, "CreateDate")?,
            };
            Ok((
                Slot::Value(data),
                complete(
                    row,
                    &[("DeletionTaskId", Shape::Text), ("CreateDate", Shape::Text)],
                ),
            ))
        }
        _ => anyhow::bail!("Alibaba role deletion observation shape refused"),
    }
}
