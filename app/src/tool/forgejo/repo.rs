use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::Seat;

impl Seat {
    pub(super) fn home(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send("GET", &format!("{}/repos/{owner}/{name}", self.base), None)
    }

    pub(super) fn provisioned(&self) -> Result<Value> {
        let Some(raw) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo repo create requires --body");
        };
        let mut body: Value = serde_json::from_str(raw).context("invalid --body")?;
        let Some(fields) = body.as_object_mut() else {
            bail!("@forgejo repo create --body must be a JSON object");
        };
        let (owner, name) = super::pair(&self.repo()?)?;
        if let Some(held) = fields.get("name")
            && held.as_str() != Some(&name)
        {
            bail!("@forgejo repo create --body name must match --repo");
        }
        fields.insert("name".into(), Value::String(name));
        self.send(
            "POST",
            &format!("{}/orgs/{}/repos", self.base, super::quote(&owner)),
            Some(&body),
        )
    }

    pub(super) fn patched(&self) -> Result<Value> {
        let Some(raw) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo repo edit requires --body");
        };
        let body: Value = serde_json::from_str(raw).context("invalid --body")?;
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "PATCH",
            &format!("{}/repos/{owner}/{name}", self.base),
            Some(&body),
        )
    }

    pub(super) fn removed(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "DELETE",
            &format!("{}/repos/{owner}/{name}", self.base),
            None,
        )
    }

    pub(super) fn stem(&self, id: &str) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "GET",
            &format!(
                "{}/repos/{owner}/{name}/branches/{}",
                self.base,
                super::quote(id)
            ),
            None,
        )
    }

    pub(super) fn forked(&self, name: &str) -> Result<Value> {
        let Some(old) = self.flag("from").filter(|held| !held.is_empty()) else {
            bail!("@forgejo branch create requires --from");
        };
        let (owner, repo) = super::pair(&self.repo()?)?;
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{repo}/branches", self.base),
            Some(&json!({"new_branch_name": name, "old_branch_name": old})),
        )
    }

    pub(super) fn guard(&self, id: &str) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "GET",
            &format!(
                "{}/repos/{owner}/{name}/branch_protections/{}",
                self.base,
                super::quote(id)
            ),
            None,
        )
    }

    pub(super) fn shield(&self) -> Result<Value> {
        let body = self.object()?;
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "POST",
            &format!("{}/repos/{owner}/{name}/branch_protections", self.base),
            Some(&body),
        )
    }

    pub(super) fn ruled(&self, id: &str) -> Result<Value> {
        let body = self.object()?;
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "PATCH",
            &format!(
                "{}/repos/{owner}/{name}/branch_protections/{}",
                self.base,
                super::quote(id)
            ),
            Some(&body),
        )
    }

    pub(super) fn secrets(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        let value = self.send(
            "GET",
            &format!("{}/repos/{owner}/{name}/actions/secrets", self.base),
            None,
        )?;
        Ok(value.get("secrets").cloned().unwrap_or(value))
    }

    pub(super) fn stored(&self, id: &str) -> Result<Value> {
        let Some(data) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo secret set requires --body");
        };
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "PUT",
            &format!(
                "{}/repos/{owner}/{name}/actions/secrets/{}",
                self.base,
                super::quote(id)
            ),
            Some(&json!({ "data": data })),
        )
    }

    pub(super) fn cleared(&self, id: &str) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.send(
            "DELETE",
            &format!(
                "{}/repos/{owner}/{name}/actions/secrets/{}",
                self.base,
                super::quote(id)
            ),
            None,
        )
    }

    pub(super) fn labels(&self) -> Result<Value> {
        let (owner, name) = super::pair(&self.repo()?)?;
        self.pages(&format!(
            "{}/repos/{owner}/{name}/labels?limit=50",
            self.base
        ))
    }

    fn object(&self) -> Result<Value> {
        let Some(raw) = self.flag("body").filter(|held| !held.is_empty()) else {
            bail!("@forgejo protection requires --body");
        };
        serde_json::from_str(raw).context("invalid --body")
    }
}
