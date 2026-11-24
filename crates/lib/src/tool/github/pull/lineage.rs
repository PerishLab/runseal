use super::{Call, Cause, Client, Error, Receipt, Tree, sha};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Comparison {
    pub url: String,
    #[serde(rename = "base_commit")]
    pub base: Tree,
    #[serde(rename = "merge_base_commit")]
    pub merge: Tree,
    pub status: State,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Ahead,
    Behind,
    Diverged,
    Identical,
}

impl Client {
    pub fn ancestry(
        &self,
        repository: (&str, &str),
        base: &str,
        head: &str,
    ) -> Result<Receipt<Comparison>, Error> {
        if !sha(base) || !sha(head) {
            return Err(Error::local(Cause::Input));
        }
        let repo = super::repository(repository.0, repository.1)?;
        let base = base.to_ascii_lowercase();
        let head = head.to_ascii_lowercase();
        let route = format!("/repos/{repo}/compare/{base}...{head}");
        let url = format!("https://api.github.com{route}");
        let path = format!("{route}?per_page=1&page=1");
        self.wire.send(&self.bearer, Call::Source(&path), |value| {
            let found: Comparison = serde_json::from_value(value).ok()?;
            let exact = found.url.eq_ignore_ascii_case(&url)
                && found.base.sha.eq_ignore_ascii_case(&base)
                && sha(&found.merge.sha);
            exact.then_some(found)
        })
    }
}
