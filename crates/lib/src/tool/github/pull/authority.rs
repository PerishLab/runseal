use super::{Call, Cause, Client, Error, Receipt, Target, node, repository, sha, valid};
use crate::github::{Window, graph};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MANIFEST: &str = "query RunsealManifest($owner:String!,$name:String!){repository(owner:$owner,name:$name){id object(expression:\"HEAD:plumb.toml\"){... on Blob{oid text}}}}";
const CLOSING: &str = "query RunsealClosing($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String){repository(owner:$owner,name:$name){id pull:pullRequest(number:$number){id number url head:headRefOid body state updated:updatedAt references:closingIssuesReferences(first:$size,after:$after){total:totalCount nodes{id number url repository{name owner{login}}} page:pageInfo{next:hasNextPage cursor:endCursor}}}}}";

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub repository: String,
    pub oid: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Closing {
    pub repository: String,
    pub pull: Value,
}

#[derive(Deserialize)]
struct Blob {
    oid: String,
    text: Option<String>,
}

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

    pub fn manifest(&self, owner: &str, name: &str) -> Result<Receipt<Manifest>, Error> {
        if !valid(owner, false) || !valid(name, true) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner,"name":name});
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(MANIFEST, variables)),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let id = repo.get("id")?.as_str()?;
                if !node(id) {
                    return None;
                }
                let object = repo.get("object")?;
                if object.is_null() {
                    return Some(Manifest {
                        repository: id.into(),
                        oid: None,
                        text: None,
                    });
                }
                let blob = Blob::deserialize(object).ok()?;
                if blob.oid.is_empty() {
                    return None;
                }
                Some(Manifest {
                    repository: id.into(),
                    oid: Some(blob.oid),
                    text: blob.text,
                })
            },
        )
    }

    pub fn closing(&self, target: &Target, window: &Window) -> Result<Receipt<Closing>, Error> {
        let mut variables = target.variables();
        variables["size"] = json!(window.size);
        variables["after"] = json!(window.after);
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(CLOSING, variables)),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let id = repo.get("id")?.as_str()?;
                let pull = repo.get("pull")?;
                if !node(id) || pull.is_null() {
                    return None;
                }
                let number = pull.get("number")?.as_u64()?;
                (number == u64::from(target.number())).then(|| Closing {
                    repository: id.into(),
                    pull: pull.clone(),
                })
            },
        )
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
