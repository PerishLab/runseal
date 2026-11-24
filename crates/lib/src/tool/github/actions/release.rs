use super::archive::{Archive, Download, failed, location, stage, storage};
use super::{Call, Cause, Client, Error, Receipt, node, reference, repository};
use crate::github::{Evidence, Outcome};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use url::{Position, Url};

#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub id: u64,
    #[serde(rename = "node_id")]
    pub node: String,
    #[serde(rename = "tag_name")]
    pub tag: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub immutable: Option<bool>,
    #[serde(rename = "published_at")]
    pub published: Option<String>,
    pub url: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    pub id: u64,
    #[serde(rename = "node_id")]
    pub node: String,
    pub name: String,
    pub size: u64,
    pub state: String,
    #[serde(rename = "content_type")]
    pub media: String,
    pub digest: Option<String>,
    pub url: String,
    #[serde(rename = "browser_download_url")]
    pub browser: String,
}

impl Client {
    pub fn fetch(
        &self,
        owner: &str,
        name: &str,
        download: &Download,
        target: &Path,
    ) -> Result<Receipt<Archive>, Error> {
        let repository = repository(owner, name)?;
        let file = stage(target)?;
        let url = format!(
            "https://api.github.com/repos/{repository}/releases/assets/{}",
            download.id
        );
        let mut evidence = Evidence {
            attempts: 1,
            ..Evidence::default()
        };
        let agent = storage();
        let first = agent
            .get(url)
            .header("authorization", self.bearer.as_str())
            .header("accept", "application/octet-stream")
            .header("user-agent", "perishlab-runseal")
            .header("x-github-api-version", "2022-11-28")
            .call()
            .map_err(|_| failed(Cause::Network, &evidence))?;
        let header = |name| {
            first
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
        };
        evidence.status = Some(first.status().as_u16());
        evidence.request = crate::github::wire::identifier(header("x-github-request-id"));
        evidence.wait = header("retry-after").and_then(|value| value.parse().ok());
        evidence.reset = header("x-ratelimit-reset").and_then(|value| value.parse().ok());
        let exhausted = header("x-ratelimit-remaining") == Some("0");
        let mut response = match first.status().as_u16() {
            200 => first,
            302 => redirect(&agent, &first, &evidence)?,
            403 if exhausted || evidence.wait.is_some() => {
                return Err(Error {
                    outcome: Outcome::Refused,
                    cause: Cause::Limited,
                    evidence,
                });
            }
            400 | 401 | 403 | 404 | 410 | 422 => {
                return Err(Error {
                    outcome: Outcome::Refused,
                    cause: Cause::Http,
                    evidence,
                });
            }
            429 => {
                return Err(Error {
                    outcome: Outcome::Refused,
                    cause: Cause::Limited,
                    evidence,
                });
            }
            _ => return Err(failed(Cause::Http, &evidence)),
        };
        download.publish(&mut response, file, target, evidence)
    }

    pub fn release(&self, owner: &str, name: &str, tag: &str) -> Result<Receipt<Release>, Error> {
        let repository = repository(owner, name)?;
        if !reference(tag) {
            return Err(Error::local(Cause::Input));
        }
        let segments = ["releases", "tags", tag];
        let base = format!("https://api.github.com/repos/{repository}");
        let endpoint = address(&base, &segments).ok_or_else(|| Error::local(Cause::Input))?;
        let path = &endpoint[Position::BeforePath..];
        self.wire.send(&self.bearer, Call::Source(path), |value| {
            let found: Release = serde_json::from_value(value).ok()?;
            found.matches(&repository, tag).then_some(found)
        })
    }

    pub fn asset(&self, owner: &str, name: &str, id: u64) -> Result<Receipt<Asset>, Error> {
        let repository = repository(owner, name)?;
        if id == 0 {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repository}/releases/assets/{id}");
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Asset = serde_json::from_value(value).ok()?;
            (found.id == id && found.matches(&repository)).then_some(found)
        })
    }
}

impl Release {
    fn matches(&self, repository: &str, tag: &str) -> bool {
        if self.id == 0 || !node(&self.node) || self.tag != tag {
            return false;
        }
        let api = format!(
            "https://api.github.com/repos/{repository}/releases/{}",
            self.id
        );
        if !self.url.eq_ignore_ascii_case(&api) {
            return false;
        }
        let mut ids = BTreeSet::new();
        self.assets.iter().all(|asset| {
            ids.insert(asset.id) && asset.matches(repository) && asset.tag(repository, tag)
        })
    }
}

impl Asset {
    fn matches(&self, repository: &str) -> bool {
        if self.id == 0 || !node(&self.node) || self.name.is_empty() {
            return false;
        }
        let api = format!(
            "https://api.github.com/repos/{repository}/releases/assets/{}",
            self.id
        );
        if !self.url.eq_ignore_ascii_case(&api) {
            return false;
        }
        let Some((base, _)) = self.browser.split_once("/releases/download/") else {
            return false;
        };
        if !base.eq_ignore_ascii_case(&format!("https://github.com/{repository}")) {
            return false;
        }
        let Ok(browser) = Url::parse(&self.browser) else {
            return false;
        };
        let escaped = address("https://github.com", &[&self.name]);
        let file = browser.path().rsplit('/').next();
        let expected = escaped
            .as_ref()
            .and_then(|url| url.path().rsplit('/').next());
        browser.query().is_none() && browser.fragment().is_none() && file == expected
    }

    fn tag(&self, repository: &str, tag: &str) -> bool {
        let base = format!("https://github.com/{repository}");
        let Some(expected) = address(&base, &["releases", "download", tag, &self.name]) else {
            return false;
        };
        let expected = expected.as_str().split_once("/releases/download/");
        self.browser
            .split_once("/releases/download/")
            .map(|(_, path)| path)
            == expected.map(|(_, path)| path)
    }
}

fn address(base: &str, segments: &[&str]) -> Option<Url> {
    let mut url = Url::parse(base).ok()?;
    url.path_segments_mut()
        .ok()?
        .extend(segments.iter().copied());
    Some(url)
}

fn redirect(
    agent: &ureq::Agent,
    response: &ureq::http::Response<ureq::Body>,
    evidence: &Evidence,
) -> Result<ureq::http::Response<ureq::Body>, Error> {
    let mut locations = response.headers().get_all("location").iter();
    let url = locations
        .next()
        .and_then(|value| value.to_str().ok())
        .and_then(destination)
        .ok_or_else(|| failed(Cause::Shape, evidence))?;
    if locations.next().is_some() {
        return Err(failed(Cause::Shape, evidence));
    }
    agent
        .get(url.as_str())
        .call()
        .map_err(|_| failed(Cause::Network, evidence))
}

fn destination(value: &str) -> Option<Url> {
    let url = location(value)?;
    (url.domain() == Some("release-assets.githubusercontent.com")).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::destination;

    #[test]
    fn storage() {
        assert!(
            destination("https://release-assets.githubusercontent.com/a?signature=example")
                .is_some()
        );
        assert!(
            destination("https://release-assets.githubusercontent.com.other.invalid/a").is_none()
        );
        assert!(destination("http://release-assets.githubusercontent.com/a").is_none());
    }
}
