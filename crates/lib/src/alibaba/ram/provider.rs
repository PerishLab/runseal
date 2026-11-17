use super::Slot;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub issuer: String,
    pub clients: Vec<String>,
    pub fingerprints: Vec<String>,
    pub hours: u64,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Data {
    pub name: String,
    pub arn: String,
    pub issuer: String,
    pub clients: Slot<String>,
    pub fingerprints: Slot<String>,
    pub hours: Slot<u64>,
    pub description: Slot<String>,
}

pub(super) fn decode(
    value: &serde_json::Value,
    name: &str,
    account: &str,
) -> anyhow::Result<(Data, bool)> {
    use super::project::{Shape, arn, complete, slot, text};
    let observed = text(value, "OIDCProviderName")?;
    if observed != name {
        anyhow::bail!("Alibaba OIDC provider response names a different target");
    }
    let resource = text(value, "Arn")?;
    arn(resource, account, "oidc-provider", name)?;
    let issuer = text(value, "IssuerUrl")?;
    super::check::Text(issuer).issuer()?;
    let data = Data {
        name: observed.into(),
        arn: resource.into(),
        issuer: issuer.into(),
        clients: slot(value, "ClientIds")?,
        fingerprints: slot(value, "Fingerprints")?,
        hours: slot(value, "IssuanceLimitTime")?,
        description: slot(value, "Description")?,
    };
    let known = complete(
        value,
        &[
            ("OIDCProviderName", Shape::Text),
            ("Arn", Shape::Text),
            ("IssuerUrl", Shape::Text),
            ("ClientIds", Shape::Text),
            ("Fingerprints", Shape::Text),
            ("IssuanceLimitTime", Shape::Number),
            ("Description", Shape::Text),
            ("CreateDate", Shape::Text),
            ("UpdateDate", Shape::Text),
            ("GmtCreate", Shape::Text),
            ("GmtModified", Shape::Text),
        ],
    );
    Ok((data, known))
}
