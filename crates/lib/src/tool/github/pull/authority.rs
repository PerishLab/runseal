use super::{Call, Cause, Client, Error, Receipt, repository, sha, valid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Branch {
    pub name: String,
    pub protected: bool,
    pub commit: Head,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Head {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Permission {
    pub permission: String,
    #[serde(rename = "role_name")]
    pub role: String,
    pub user: Principal,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Principal {
    pub id: u64,
    pub login: String,
    #[serde(rename = "node_id")]
    pub node: String,
    #[serde(rename = "type")]
    pub kind: String,
}

impl Principal {
    pub(in crate::tool::github) fn matches(&self, login: &str) -> bool {
        self.id != 0 && super::node(&self.node) && self.login.eq_ignore_ascii_case(login)
    }
}

impl Client {
    pub fn branch(&self, owner: &str, name: &str, branch: &str) -> Result<Receipt<Branch>, Error> {
        let repo = repository(owner, name)?;
        if !super::reference(branch) {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repo}/branches/{branch}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Branch = serde_json::from_value(value).ok()?;
            found.matches(&repo, branch).then_some(found)
        })
    }

    pub fn permission(
        &self,
        owner: &str,
        name: &str,
        login: &str,
    ) -> Result<Receipt<Permission>, Error> {
        let repo = repository(owner, name)?;
        if !valid(login, false) {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repo}/collaborators/{login}/permission");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Permission = serde_json::from_value(value).ok()?;
            found.matches(login).then_some(found)
        })
    }
}

impl Branch {
    fn matches(&self, repository: &str, branch: &str) -> bool {
        if self.name != branch || !sha(&self.commit.sha) {
            return false;
        }
        let url = format!(
            "https://api.github.com/repos/{repository}/commits/{}",
            self.commit.sha
        );
        self.commit.url.eq_ignore_ascii_case(&url)
    }
}

impl Permission {
    fn matches(&self, login: &str) -> bool {
        self.user.matches(login) && !self.permission.is_empty() && !self.role.is_empty()
    }
}
