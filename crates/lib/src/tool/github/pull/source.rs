use super::{Call, Cause, Client, Error, Receipt, repository, sha};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize, Serializer};
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use url::{Position, Url};

#[derive(Debug, Deserialize, Serialize)]
pub struct Directory {
    pub sha: String,
    pub url: String,
    pub truncated: bool,
    #[serde(rename = "tree")]
    pub rows: Vec<Member>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Member {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub sha: String,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Blob {
    pub sha: String,
    pub size: u64,
    #[serde(rename = "base64", serialize_with = "encode")]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct File {
    pub path: String,
    pub commit: String,
    #[serde(flatten)]
    pub blob: Blob,
}

#[derive(Deserialize)]
struct Contents {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)]
    encoded: Encoded,
}

#[derive(Deserialize)]
struct Encoded {
    sha: String,
    url: String,
    size: u64,
    encoding: String,
    content: String,
}

impl Client {
    pub fn tree(
        &self,
        owner: &str,
        name: &str,
        id: &str,
        recursive: bool,
    ) -> Result<Receipt<Directory>, Error> {
        let repo = repository(owner, name)?;
        let id = identifier(id)?;
        let address = format!("/repos/{repo}/git/trees/{id}");
        let url = format!("https://api.github.com{address}");
        let path = if recursive {
            format!("{address}?recursive=1")
        } else {
            address
        };
        self.wire.send(&self.bearer, Call::Source(&path), |value| {
            let found: Directory = serde_json::from_value(value).ok()?;
            found.matches(&id, &url, recursive).then_some(found)
        })
    }

    pub fn contents(
        &self,
        owner: &str,
        name: &str,
        commit: &str,
        path: &str,
    ) -> Result<Receipt<File>, Error> {
        let repo = repository(owner, name)?;
        let commit = identifier(commit)?;
        let url = location(&repo, &commit, path).ok_or_else(|| Error::local(Cause::Input))?;
        let address = &url[Position::BeforePath..];
        self.wire
            .send(&self.bearer, Call::Source(address), |value| {
                let found: Contents = serde_json::from_value(value).ok()?;
                if found.kind != "file" || found.path != path {
                    return None;
                }
                let suffix = url.as_str().split_once("/contents/")?;
                let actual = found.encoded.url.split_once("/contents/")?;
                if !actual.0.eq_ignore_ascii_case(suffix.0) || actual.1 != suffix.1 {
                    return None;
                }
                let blob = found.encoded.bytes()?;
                Some(File {
                    path: found.path,
                    commit: commit.clone(),
                    blob,
                })
            })
    }

    pub fn blob(&self, owner: &str, name: &str, id: &str) -> Result<Receipt<Blob>, Error> {
        let repo = repository(owner, name)?;
        let id = identifier(id)?;
        let path = format!("/repos/{repo}/git/blobs/{id}");
        let url = format!("https://api.github.com{path}");
        self.wire.send(&self.bearer, Call::Source(&path), |value| {
            serde_json::from_value::<Encoded>(value)
                .ok()?
                .decode(&id, &url)
        })
    }
}

impl Directory {
    fn matches(&self, id: &str, url: &str, recursive: bool) -> bool {
        if !self.sha.eq_ignore_ascii_case(id) || !self.url.eq_ignore_ascii_case(url) {
            return false;
        }
        let mut paths = BTreeSet::new();
        self.rows
            .iter()
            .all(|row| paths.insert(&row.path) && row.valid(recursive))
    }
}

impl Member {
    fn valid(&self, recursive: bool) -> bool {
        if !sha(&self.sha) || self.path.contains('\0') {
            return false;
        }
        if !recursive && self.path.contains('/') {
            return false;
        }
        if !self
            .path
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
        {
            return false;
        }
        match (self.kind.as_str(), self.mode.as_str()) {
            ("blob", "100644" | "100755" | "120000") => self.size.is_some(),
            ("tree", "040000") | ("commit", "160000") => true,
            _ => false,
        }
    }
}

impl Encoded {
    fn decode(self, id: &str, url: &str) -> Option<Blob> {
        if !self.sha.eq_ignore_ascii_case(id) || !self.url.eq_ignore_ascii_case(url) {
            return None;
        }
        self.bytes()
    }

    fn bytes(self) -> Option<Blob> {
        if self.encoding != "base64" {
            return None;
        }
        let encoded = self.content.lines().collect::<String>();
        let bytes = STANDARD.decode(encoded).ok()?;
        if bytes.len() as u64 != self.size {
            return None;
        }
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", self.size));
        hash.update(&bytes);
        if !self
            .sha
            .eq_ignore_ascii_case(&format!("{:x}", hash.finalize()))
        {
            return None;
        }
        Some(Blob {
            sha: self.sha,
            size: self.size,
            bytes,
        })
    }
}

fn location(repository: &str, commit: &str, path: &str) -> Option<Url> {
    if path.contains('\0') || !path.split('/').all(|part| !matches!(part, "" | "." | "..")) {
        return None;
    }
    let mut url = Url::parse(&format!(
        "https://api.github.com/repos/{repository}/contents/"
    ))
    .ok()?;
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .extend(path.split('/'));
    url.query_pairs_mut().append_pair("ref", commit);
    Some(url)
}

fn identifier(id: &str) -> Result<String, Error> {
    if !sha(id) {
        return Err(Error::local(Cause::Input));
    }
    Ok(id.to_ascii_lowercase())
}

fn encode<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}
