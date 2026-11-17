use super::Slot;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub document: Value,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Data {
    pub name: String,
    pub kind: String,
    pub version: Slot<String>,
    pub attachments: Slot<u64>,
    pub description: Slot<String>,
    pub document: Slot<Version>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Version {
    pub id: String,
    pub default: Slot<bool>,
    pub document: Slot<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub name: String,
    pub kind: String,
    pub version: Slot<String>,
    pub attached: Slot<String>,
}

pub(super) fn decode(body: &Value, name: &str) -> anyhow::Result<(Data, bool)> {
    use super::project::{Shape, complete, object, slot, text};
    let value = object(body, "Policy")?;
    if text(value, "PolicyName")? != name || text(value, "PolicyType")? != "Custom" {
        anyhow::bail!("Alibaba policy response names a different target or policy kind");
    }
    let (document, nested) = document(body)?;
    let version = slot(value, "DefaultVersion")?;
    reconcile(&version, &document)?;
    let data = Data {
        name: name.into(),
        kind: "Custom".into(),
        version,
        attachments: slot(value, "AttachmentCount")?,
        description: slot(value, "Description")?,
        document,
    };
    let known = complete(
        value,
        &[
            ("PolicyName", Shape::Text),
            ("PolicyType", Shape::Text),
            ("DefaultVersion", Shape::Text),
            ("AttachmentCount", Shape::Number),
            ("Description", Shape::Text),
            ("PolicyDocument", Shape::Text),
            ("CreateDate", Shape::Text),
            ("UpdateDate", Shape::Text),
        ],
    );
    Ok((data, known && nested))
}

fn reconcile(version: &Slot<String>, document: &Slot<Version>) -> anyhow::Result<()> {
    if let (Slot::Value(id), Slot::Value(document)) = (version, document)
        && id != &document.id
    {
        anyhow::bail!("Alibaba default policy version observations disagree");
    }
    Ok(())
}

fn document(body: &Value) -> anyhow::Result<(Slot<Version>, bool)> {
    match body.get("DefaultPolicyVersion") {
        None => Ok((Slot::Missing, true)),
        Some(Value::Null) => Ok((Slot::Null, true)),
        Some(row) => {
            let (data, known) = version(row)?;
            if matches!(data.default, Slot::Value(false)) {
                anyhow::bail!("Alibaba default policy version is explicitly nondefault");
            }
            Ok((Slot::Value(data), known))
        }
    }
}

pub(super) fn version(value: &Value) -> anyhow::Result<(Version, bool)> {
    use super::project::{Shape, complete, slot, text};
    let id = text(value, "VersionId")?;
    super::check::Text(id).version()?;
    let data = Version {
        id: id.into(),
        default: slot(value, "IsDefaultVersion")?,
        document: slot(value, "PolicyDocument")?,
    };
    let known = complete(
        value,
        &[
            ("VersionId", Shape::Text),
            ("IsDefaultVersion", Shape::Boolean),
            ("PolicyDocument", Shape::Text),
            ("CreateDate", Shape::Text),
        ],
    );
    Ok((data, known))
}

pub(super) fn entries(body: &Value) -> anyhow::Result<(Vec<Entry>, bool)> {
    use super::project::{Shape, complete, object, slot, text};
    let value = object(body, "Policies")?;
    let list = value
        .get("Policy")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Alibaba role policy list absent, null or malformed"))?;
    let mut known = complete(value, &[("Policy", Shape::Array)]);
    let mut seen = std::collections::BTreeSet::new();
    let mut entries = Vec::new();
    for row in list {
        let name = text(row, "PolicyName")?;
        let kind = text(row, "PolicyType")?;
        if !seen.insert((name, kind)) {
            anyhow::bail!("Alibaba role policy list repeats a target");
        }
        known &= matches!(kind, "Custom" | "System")
            && complete(
                row,
                &[
                    ("PolicyName", Shape::Text),
                    ("PolicyType", Shape::Text),
                    ("DefaultVersion", Shape::Text),
                    ("Description", Shape::Text),
                    ("AttachDate", Shape::Text),
                ],
            );
        entries.push(Entry {
            name: name.into(),
            kind: kind.into(),
            version: slot(row, "DefaultVersion")?,
            attached: slot(row, "AttachDate")?,
        });
    }
    Ok((entries, known))
}
