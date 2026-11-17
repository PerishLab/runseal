use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub source: String,
    pub issuer: String,
    pub subject: String,
    pub audience: String,
    pub account: String,
    pub provider: String,
    pub role: String,
    pub id: String,
    pub session: String,
    pub duration: u64,
}

impl Binding {
    pub fn validate(&self) -> Result<()> {
        endpoint(&self.source, false)?;
        endpoint(&self.issuer, true)?;
        for value in [&self.subject, &self.audience] {
            super::super::model::text(value)?;
        }
        if self.audience.len() > 255 || self.audience.contains(',') {
            bail!("Alibaba federation requires one bounded, unambiguous audience");
        }
        for value in [&self.account, &self.id] {
            if value.starts_with('0') || value.parse::<u64>().ok().is_none_or(|v| v == 0) {
                bail!("Alibaba federation requires explicit positive account and RoleId");
            }
            if !value.bytes().all(|v| v.is_ascii_digit()) {
                bail!("Alibaba federation numeric identity refused");
            }
        }
        resource(&self.provider, &self.account, "oidc-provider")?;
        resource(&self.role, &self.account, "role")?;
        if !(2..=64).contains(&self.session.len())
            || !self
                .session
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'-' | b'_' | b'.' | b'@'))
        {
            bail!("Alibaba role session name refused");
        }
        if !(900..=43200).contains(&self.duration) {
            bail!("Alibaba role duration must be explicit within 900..43200 seconds");
        }
        Ok(())
    }

    pub(super) fn caller(&self) -> String {
        let name = self
            .role
            .strip_prefix(&format!("acs:ram::{}:role/", self.account))
            .expect("validated role");
        format!(
            "acs:ram::{}:assumed-role/{}/{}",
            self.account, name, self.session
        )
    }

    pub(super) fn arn(&self) -> String {
        format!("{}/{}", self.role, self.session)
    }
}

fn endpoint(value: &str, canonical: bool) -> Result<()> {
    let url = url::Url::parse(value)?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !matches!(
        (url.scheme(), canonical, local),
        ("https", _, _) | ("http", false, true)
    ) {
        bail!("Ensign binding requires HTTPS, or an explicit loopback HTTP source alias");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Ensign binding cannot carry URL credentials");
    }
    if url.host_str().is_none() || url.query().is_some() || url.fragment().is_some() {
        bail!("Ensign binding requires an exact API root without query or fragment");
    }
    if value.ends_with('/') {
        bail!("Ensign API root cannot have a trailing slash");
    }
    Ok(())
}

fn resource(value: &str, account: &str, kind: &str) -> Result<()> {
    let prefix = format!("acs:ram::{account}:{kind}/");
    let name = value.strip_prefix(&prefix).ok_or_else(|| {
        anyhow::anyhow!("Alibaba resource owner or kind differs from declared binding")
    })?;
    if name.is_empty() || name.len() > 128 || !label(name) {
        bail!("Alibaba federation resource name refused");
    }
    Ok(())
}

fn label(value: &str) -> bool {
    value
        .bytes()
        .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'-' | b'_' | b'.'))
}
