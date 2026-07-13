use super::{Cmd, Config, cmd};
use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;
pub(super) fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "list" => cmd(args).list(),
        "create" => cmd(args).create(),
        "update" => cmd(args).update(),
        _ => bail!("usage: runseal @tool cloudflare zone dns-record list|create|update ..."),
    }
}
fn result(payload: Option<String>) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
    Ok(Some(serde_json::to_string(
        value.get("result").unwrap_or(&JsonValue::Null),
    )?))
}

impl Cmd<'_> {
    fn list(&self) -> Result<Option<String>> {
        let zone = self.required("--zone-id")?;
        let query = self
            .optional("--name")
            .map(|name| vec![("name".to_string(), name)])
            .unwrap_or_default();
        let config = Config::load()?;
        let payload = config.request("GET", &format!("/zones/{zone}/dns_records"), query, None)?;
        let value: JsonValue = serde_json::from_str(&payload.unwrap_or_default())?;
        Ok(Some(serde_json::to_string(
            value.get("result").unwrap_or(&JsonValue::Array(Vec::new())),
        )?))
    }

    fn create(&self) -> Result<Option<String>> {
        let zone = self.required("--zone-id")?;
        let body = self.payload()?;
        let config = Config::load()?;
        let payload = config.request(
            "POST",
            &format!("/zones/{zone}/dns_records"),
            Vec::new(),
            Some(body),
        )?;
        result(payload)
    }

    fn update(&self) -> Result<Option<String>> {
        let zone = self.required("--zone-id")?;
        let record = self.required("--record-id")?;
        let body = self.payload()?;
        let config = Config::load()?;
        let payload = config.request(
            "PATCH",
            &format!("/zones/{zone}/dns_records/{record}"),
            Vec::new(),
            Some(body),
        )?;
        result(payload)
    }

    fn payload(&self) -> Result<JsonValue> {
        let raw = self.required("--json")?;
        serde_json::from_str(&raw).context("invalid DNS record JSON")
    }
}
