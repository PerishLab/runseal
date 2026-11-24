use super::{Call, Cause, Client, Error, Receipt, repository, sha};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub sha: String,
    pub tree: Tree,
    pub parents: Vec<Tree>,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tree {
    pub sha: String,
}

impl Client {
    pub fn commit(&self, owner: &str, name: &str, head: &str) -> Result<Receipt<Commit>, Error> {
        let repo = repository(owner, name)?;
        if !sha(head) {
            return Err(Error::local(Cause::Input));
        }
        let head = head.to_ascii_lowercase();
        let path = format!("/repos/{repo}/git/commits/{head}");
        let expected = format!("https://api.github.com{path}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Commit = serde_json::from_value(value).ok()?;
            let matches = found.sha.eq_ignore_ascii_case(&head)
                && found.url.eq_ignore_ascii_case(&expected)
                && sha(&found.tree.sha);
            (matches && found.parents.iter().all(|parent| sha(&parent.sha))).then_some(found)
        })
    }
}
