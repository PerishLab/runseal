use super::{Call, Cause, Client, Error, Receipt, repository, sha};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum Reference {
    Branch(String),
    Tag(String),
}

impl Reference {
    fn selector(&self) -> Result<String, Error> {
        let (namespace, name) = match self {
            Self::Branch(name) => ("heads", name),
            Self::Tag(name) => ("tags", name),
        };
        if !super::reference(name) {
            return Err(Error::local(Cause::Input));
        }
        Ok(format!("{namespace}/{name}"))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pointer {
    #[serde(rename = "ref")]
    pub reference: String,
    pub url: String,
    pub object: Object,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Object {
    pub sha: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Annotation {
    pub sha: String,
    pub tag: String,
    pub url: String,
    pub object: Object,
}

impl Client {
    pub fn annotation(
        &self,
        owner: &str,
        name: &str,
        id: &str,
    ) -> Result<Receipt<Annotation>, Error> {
        let repo = repository(owner, name)?;
        if !sha(id) {
            return Err(Error::local(Cause::Input));
        }
        let id = id.to_ascii_lowercase();
        let path = format!("/repos/{repo}/git/tags/{id}");
        let url = format!("https://api.github.com{path}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Annotation = serde_json::from_value(value).ok()?;
            if !found.sha.eq_ignore_ascii_case(&id) || !found.url.eq_ignore_ascii_case(&url) {
                return None;
            }
            (!found.tag.is_empty() && found.object.valid()).then_some(found)
        })
    }

    pub fn matching(
        &self,
        owner: &str,
        name: &str,
        prefix: &str,
    ) -> Result<Receipt<Vec<Pointer>>, Error> {
        let repo = repository(owner, name)?;
        if !super::reference(prefix.strip_suffix('/').unwrap_or(prefix)) {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repo}/git/matching-refs/heads/{prefix}");
        let origin = format!("https://api.github.com/repos/{repo}");
        let expected = format!("refs/heads/{prefix}");
        self.wire
            .inspect(&self.bearer, Call::Source(&path), |value, response| {
                if response.headers.contains_key("link") {
                    return None;
                }
                let found: Vec<Pointer> = serde_json::from_value(value).ok()?;
                let mut seen = BTreeSet::new();
                found
                    .iter()
                    .all(|row| row.prefix(&expected, &origin) && seen.insert(&row.reference))
                    .then_some(found)
            })
    }

    pub fn reference(
        &self,
        owner: &str,
        name: &str,
        selection: &Reference,
    ) -> Result<Receipt<Pointer>, Error> {
        let repo = repository(owner, name)?;
        let selector = selection.selector()?;
        let path = format!("/repos/{repo}/git/ref/{selector}");
        let expected = format!("refs/{selector}");
        let origin = format!("https://api.github.com/repos/{repo}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Pointer = serde_json::from_value(value).ok()?;
            found
                .matches(selection, &expected, &origin)
                .then_some(found)
        })
    }
}

impl Pointer {
    fn matches(&self, selection: &Reference, expected: &str, origin: &str) -> bool {
        if self.reference != expected || !self.bound(origin) {
            return false;
        }
        match selection {
            Reference::Branch(_) => self.object.kind == "commit",
            Reference::Tag(_) => self.object.valid(),
        }
    }

    fn prefix(&self, expected: &str, origin: &str) -> bool {
        if !self.reference.starts_with(expected) || self.object.kind != "commit" {
            return false;
        }
        self.bound(origin)
            && self
                .reference
                .strip_prefix("refs/heads/")
                .is_some_and(super::reference)
    }

    fn bound(&self, origin: &str) -> bool {
        if !sha(&self.object.sha) {
            return false;
        }
        let Some((repository, reference)) = self.url.split_once("/git/refs/") else {
            return false;
        };
        repository.eq_ignore_ascii_case(origin)
            && self.reference.strip_prefix("refs/") == Some(reference)
    }
}

impl Object {
    fn valid(&self) -> bool {
        sha(&self.sha) && matches!(self.kind.as_str(), "tag" | "commit" | "blob" | "tree")
    }
}
