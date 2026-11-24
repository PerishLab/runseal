use super::{Cause, Client, Error, Receipt, repository};
use crate::github::{Evidence, Outcome};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use tempfile::NamedTempFile;
use url::Url;

const MAXIMUM: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Download {
    pub(super) id: u64,
    size: u64,
    digest: String,
}

#[derive(Debug, Serialize)]
pub struct Archive {
    pub id: u64,
    pub size: u64,
    pub digest: String,
}

impl Download {
    pub fn new(id: u64, size: u64, digest: &str) -> Result<Self, Error> {
        let hash = digest.strip_prefix("sha256:").ok_or_else(input)?;
        if id == 0 || size == 0 || size > MAXIMUM {
            return Err(input());
        }
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(input());
        }
        Ok(Self {
            id,
            size,
            digest: digest.to_ascii_lowercase(),
        })
    }
}

impl Client {
    pub fn download(
        &self,
        owner: &str,
        name: &str,
        download: &Download,
        target: &Path,
    ) -> Result<Receipt<Archive>, Error> {
        let repo = repository(owner, name)?;
        let file = stage(target)?;
        let path = format!("/repos/{repo}/actions/artifacts/{}/zip", download.id);
        let redirect = self.wire.redirect(&self.bearer, &path)?;
        let evidence = redirect.evidence;
        let failure = |cause| failed(cause, &evidence);
        if !allowed(&redirect.value) {
            return Err(failure(Cause::Shape));
        }
        let mut reply = storage()
            .get(redirect.value.as_str())
            .call()
            .map_err(|_| failure(Cause::Network))?;
        download.publish(&mut reply, file, target, evidence)
    }
}

impl Download {
    pub(super) fn publish(
        &self,
        reply: &mut ureq::http::Response<ureq::Body>,
        mut file: NamedTempFile,
        target: &Path,
        evidence: Evidence,
    ) -> Result<Receipt<Archive>, Error> {
        let failure = |cause| failed(cause, &evidence);
        if reply.status() != 200 {
            return Err(failure(Cause::Http));
        }
        let mut reader = reply.body_mut().with_config().limit(self.size + 1).reader();
        let digest = copy(&mut reader, &mut file, self.size).map_err(failure)?;
        if digest != self.digest {
            return Err(failure(Cause::Shape));
        }
        file.persist_noclobber(target)
            .map_err(|_| failure(Cause::Output))?;
        Ok(Receipt {
            outcome: Outcome::Observed,
            evidence,
            value: Archive {
                id: self.id,
                size: self.size,
                digest,
            },
        })
    }
}

pub(super) fn stage(target: &Path) -> Result<NamedTempFile, Error> {
    let parent = target.parent().ok_or_else(input)?;
    if target.file_name().is_none() {
        return Err(input());
    }
    if target.symlink_metadata().is_ok() {
        return Err(Error::local(Cause::Output));
    }
    let directory = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    NamedTempFile::new_in(directory).map_err(|_| Error::local(Cause::Output))
}

fn allowed(value: &str) -> bool {
    location(value)
        .and_then(|url| url.domain().map(host))
        .unwrap_or(false)
}

pub(super) fn location(value: &str) -> Option<Url> {
    if value.len() > 8192 || !value.starts_with("https://") || value.contains('\\') {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return None;
    }
    url.port().is_none().then_some(url)
}

fn host(host: &str) -> bool {
    host == "results-receiver.actions.githubusercontent.com"
        || host
            .strip_suffix(".blob.core.windows.net")
            .is_some_and(|name| {
                !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
}

pub(super) fn storage() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into()
}

fn copy(reader: &mut impl Read, file: &mut impl Write, size: u64) -> Result<String, Cause> {
    let mut total = 0;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|_| Cause::Read)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > size {
            return Err(Cause::Shape);
        }
        let bytes = &buffer[..count];
        file.write_all(bytes).map_err(|_| Cause::Output)?;
        hash.update(bytes);
    }
    if total != size {
        return Err(Cause::Shape);
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn input() -> Error {
    Error::local(Cause::Input)
}

pub(super) fn failed(cause: Cause, evidence: &Evidence) -> Error {
    Error {
        cause,
        outcome: Outcome::Unknown,
        evidence: evidence.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cause, allowed, copy, stage};

    #[test]
    fn location() {
        assert!(allowed("https://store.blob.core.windows.net/zip?sig=value"));
        assert!(allowed(
            "https://results-receiver.actions.githubusercontent.com/zip"
        ));
        for url in [
            "http://store.blob.core.windows.net/zip",
            "https://secret@store.blob.core.windows.net/zip",
            "https://store.blob.core.windows.net.evil.example/zip",
            "https://api.github.com/zip",
            "https://store.blob.core.windows.net:8443/zip",
        ] {
            assert!(!allowed(url), "{url}");
        }
    }

    #[test]
    fn bounds() {
        let bytes = b"abc";
        let digest = copy(&mut bytes.as_slice(), &mut Vec::new(), 3).unwrap();
        assert_eq!(
            digest,
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        for size in [2, 4] {
            assert_eq!(
                copy(&mut bytes.as_slice(), &mut Vec::new(), size),
                Err(Cause::Shape)
            );
        }
    }

    #[test]
    fn destination() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("artifact.zip");
        let staged = stage(&target).unwrap();
        std::fs::write(&target, b"owned").unwrap();
        assert!(staged.persist_noclobber(&target).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"owned");
        assert_eq!(stage(&target).unwrap_err().cause, Cause::Output);
    }
}
