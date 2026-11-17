use anyhow::{Result, bail};
use serde_json::Value;

pub(super) struct Text<'a>(pub &'a str);

impl Text<'_> {
    pub(super) fn name(&self, size: usize, extra: &[u8]) -> Result<()> {
        let value = self.0;
        if value.is_empty()
            || value.len() > size
            || !value
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || extra.contains(&v))
        {
            bail!("Alibaba RAM resource name refused");
        }
        Ok(())
    }

    pub(super) fn provider(&self) -> Result<()> {
        let value = self.0;
        self.name(128, b".-_")?;
        if !value.as_bytes()[0].is_ascii_alphanumeric()
            || !value.as_bytes()[value.len() - 1].is_ascii_alphanumeric()
        {
            bail!("Alibaba OIDC provider name refused");
        }
        Ok(())
    }

    pub(super) fn issuer(&self) -> Result<()> {
        let value = self.0;
        let url = url::Url::parse(value)?;
        if value.len() > 255 || url.scheme() != "https" || url.host_str().is_none() {
            bail!("Alibaba OIDC issuer origin refused");
        }
        if !url.username().is_empty() || url.password().is_some() {
            bail!("Alibaba OIDC issuer credentials refused");
        }
        if url.query().is_some() || url.fragment().is_some() {
            bail!("Alibaba OIDC issuer selectors refused");
        }
        Ok(())
    }

    pub(super) fn description(&self, size: usize) -> Result<()> {
        let value = self.0;
        if value.is_empty() || value.chars().count() > size || value.chars().any(char::is_control) {
            bail!("Alibaba RAM description refused");
        }
        Ok(())
    }

    pub(super) fn version(&self) -> Result<()> {
        let value = self.0;
        let number = value.strip_prefix('v').unwrap_or("");
        if number.starts_with('0')
            || !number.bytes().all(|byte| byte.is_ascii_digit())
            || number.parse::<u64>().ok().is_none_or(|v| v == 0)
        {
            bail!("Alibaba RAM policy version refused");
        }
        Ok(())
    }
}

pub(super) fn document(value: &Value) -> Result<String> {
    if !value.is_object() {
        bail!("Alibaba RAM policy must be an explicit JSON object");
    }
    let text = serde_json::to_string(value)?;
    if text.chars().count() > 6144 {
        bail!("Alibaba RAM document exceeds this bounded surface");
    }
    Ok(text)
}
