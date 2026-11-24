use super::{Call, Cause, Client, Error, Receipt, repository, sha};
use serde::{Deserialize, Serialize};
use url::form_urlencoded::Serializer;

#[derive(Clone, Debug)]
pub struct Inventory {
    run: Option<u64>,
    page: u32,
    size: u8,
    label: Option<String>,
}

impl Inventory {
    pub fn new(run: Option<u64>, page: u32, size: u8) -> Result<Self, Error> {
        if run == Some(0) || page == 0 || !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            run,
            page,
            size,
            label: None,
        })
    }

    pub fn named(mut self, label: &str) -> Result<Self, Error> {
        if label.trim().is_empty() || label.len() > 1024 || label.chars().any(char::is_control) {
            return Err(Error::local(Cause::Input));
        }
        self.label = Some(label.into());
        Ok(self)
    }

    fn query(&self) -> String {
        let mut query = Serializer::new(String::new());
        query.append_pair("per_page", &self.size.to_string());
        query.append_pair("page", &self.page.to_string());
        if let Some(label) = &self.label {
            query.append_pair("name", label);
        }
        query.finish()
    }

    fn confirms(&self, path: &str, found: &Artifacts) -> bool {
        let count = found.rows.len();
        if count > usize::from(self.size) || count as u64 > found.total {
            return false;
        }
        found.rows.iter().all(|row| {
            row.matches(path, self.run) && self.label.as_ref().is_none_or(|name| row.name == *name)
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artifacts {
    #[serde(rename = "total_count")]
    pub total: u64,
    #[serde(rename = "artifacts")]
    pub rows: Vec<Artifact>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artifact {
    pub id: u64,
    pub name: String,
    #[serde(rename = "size_in_bytes")]
    pub size: u64,
    pub expired: bool,
    pub digest: Option<String>,
    #[serde(rename = "workflow_run")]
    pub run: Option<Producer>,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Producer {
    pub id: u64,
    #[serde(rename = "repository_id")]
    pub repository: u64,
    #[serde(rename = "head_sha")]
    pub head: String,
}

impl Artifact {
    fn matches(&self, path: &str, run: Option<u64>) -> bool {
        let expected = format!("https://api.github.com{path}/{}", self.id);
        if self.id == 0 || self.name.is_empty() || !self.url.eq_ignore_ascii_case(&expected) {
            return false;
        }
        self.run.as_ref().is_none_or(|found| found.matches(run))
    }
}

impl Producer {
    fn matches(&self, run: Option<u64>) -> bool {
        if self.id == 0 || self.repository == 0 || !sha(&self.head) {
            return false;
        }
        run.is_none_or(|expected| expected == self.id)
    }
}

impl Client {
    pub fn artifacts(
        &self,
        owner: &str,
        name: &str,
        query: Inventory,
    ) -> Result<Receipt<Artifacts>, Error> {
        let repo = repository(owner, name)?;
        let artifacts = format!("/repos/{repo}/actions/artifacts");
        let scope = match query.run {
            Some(run) => format!("/repos/{repo}/actions/runs/{run}/artifacts"),
            None => artifacts.clone(),
        };
        let path = format!("{scope}?{}", query.query());
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Artifacts = serde_json::from_value(value).ok()?;
            query.confirms(&artifacts, &found).then_some(found)
        })
    }

    pub fn artifact(&self, owner: &str, name: &str, id: u64) -> Result<Receipt<Artifact>, Error> {
        let repo = repository(owner, name)?;
        if id == 0 {
            return Err(Error::local(Cause::Input));
        }
        let artifacts = format!("/repos/{repo}/actions/artifacts");
        let path = format!("{artifacts}/{id}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Artifact = serde_json::from_value(value).ok()?;
            (found.id == id && found.matches(&artifacts, None)).then_some(found)
        })
    }
}
