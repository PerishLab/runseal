use super::archive::{allowed, failed, stage, storage};
use super::{Archive, Cause, Client, Error, Receipt, repository};
use crate::github::{Evidence, Outcome};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub const INLINE: u64 = 1024 * 1024;
const CAP: u64 = 16 * 1024 * 1024;
const FILE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct Log {
    id: u64,
    limit: u64,
}

#[derive(Debug, Serialize)]
pub struct Transcript {
    pub id: u64,
    pub size: u64,
    pub digest: String,
    pub text: String,
}

impl Log {
    pub fn inline(id: u64, limit: u64) -> Result<Self, Error> {
        if id == 0 || limit == 0 || limit > CAP {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self { id, limit })
    }

    pub fn file(id: u64, limit: u64) -> Result<Self, Error> {
        if id == 0 || limit == 0 || limit > FILE {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self { id, limit })
    }

    fn read(
        self,
        reply: &mut ureq::http::Response<ureq::Body>,
        evidence: Evidence,
    ) -> Result<Receipt<Transcript>, Error> {
        let failure = |cause| failed(cause, &evidence);
        if reply.status() != 200 {
            return Err(failure(Cause::Http));
        }
        let mut reader = reply
            .body_mut()
            .with_config()
            .limit(self.limit + 1)
            .reader();
        let mut bytes = Vec::new();
        let (size, digest) = copy(&mut reader, &mut bytes, self.limit).map_err(failure)?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        Ok(Receipt {
            outcome: Outcome::Observed,
            evidence,
            value: Transcript {
                id: self.id,
                size,
                digest,
                text,
            },
        })
    }

    fn write(
        self,
        reply: &mut ureq::http::Response<ureq::Body>,
        mut file: NamedTempFile,
        target: &Path,
        evidence: Evidence,
    ) -> Result<Receipt<Archive>, Error> {
        let failure = |cause| failed(cause, &evidence);
        if reply.status() != 200 {
            return Err(failure(Cause::Http));
        }
        let mut reader = reply
            .body_mut()
            .with_config()
            .limit(self.limit + 1)
            .reader();
        let (size, digest) = copy(&mut reader, &mut file, self.limit).map_err(failure)?;
        file.persist_noclobber(target)
            .map_err(|_| failure(Cause::Output))?;
        Ok(Receipt {
            outcome: Outcome::Observed,
            evidence,
            value: Archive {
                id: self.id,
                size,
                digest,
            },
        })
    }
}

impl Client {
    pub fn logs(&self, owner: &str, name: &str, query: Log) -> Result<Receipt<Transcript>, Error> {
        if query.limit > CAP {
            return Err(Error::local(Cause::Input));
        }
        let (mut reply, evidence) = fetch(self, owner, name, query.id)?;
        query.read(&mut reply, evidence)
    }

    pub fn log(
        &self,
        owner: &str,
        name: &str,
        query: Log,
        target: &Path,
    ) -> Result<Receipt<Archive>, Error> {
        let file = stage(target)?;
        let (mut reply, evidence) = fetch(self, owner, name, query.id)?;
        query.write(&mut reply, file, target, evidence)
    }
}

fn fetch(
    client: &Client,
    owner: &str,
    name: &str,
    id: u64,
) -> Result<(ureq::http::Response<ureq::Body>, Evidence), Error> {
    let repo = repository(owner, name)?;
    let path = format!("/repos/{repo}/actions/jobs/{id}/logs");
    let redirect = client.wire.redirect(&client.bearer, &path)?;
    let evidence = redirect.evidence;
    let failure = |cause| failed(cause, &evidence);
    if !allowed(&redirect.value) {
        return Err(failure(Cause::Shape));
    }
    let reply = storage()
        .get(redirect.value.as_str())
        .call()
        .map_err(|_| failure(Cause::Network))?;
    Ok((reply, evidence))
}

fn copy(reader: &mut impl Read, sink: &mut impl Write, limit: u64) -> Result<(u64, String), Cause> {
    let mut total = 0;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|_| Cause::Read)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > limit {
            return Err(Cause::Shape);
        }
        let bytes = &buffer[..count];
        sink.write_all(bytes).map_err(|_| Cause::Output)?;
        hash.update(bytes);
    }
    Ok((total, format!("sha256:{:x}", hash.finalize())))
}

#[cfg(test)]
mod tests {
    use super::{CAP, Cause, FILE, Log, Outcome, copy, stage};
    use crate::github::{Evidence, tests::reply, tests::serve};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    fn local(status: &str, body: &str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/logs", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&stream);
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }
            stream.write_all(response.as_bytes()).unwrap();
            headers
        });
        (url, thread)
    }

    #[test]
    fn validation() {
        assert_eq!(Log::inline(0, 1).unwrap_err().cause, Cause::Input);
        assert_eq!(Log::inline(1, 0).unwrap_err().cause, Cause::Input);
        assert_eq!(Log::inline(1, CAP + 1).unwrap_err().cause, Cause::Input);
        assert_eq!(Log::file(1, FILE + 1).unwrap_err().cause, Cause::Input);
        let (size, digest) = copy(&mut b"log\n".as_slice(), &mut Vec::new(), 4).unwrap();
        assert_eq!(size, 4);
        assert!(digest.starts_with("sha256:"));
        assert_eq!(
            copy(&mut b"log\n".as_slice(), &mut Vec::new(), 3),
            Err(Cause::Shape)
        );
    }

    #[test]
    fn redirects() {
        let server = serve(vec![
            reply("302 Found", "Location: https://evil.example/logs\r\n", ""),
            reply("410 Gone", "", "expired"),
        ]);
        let query = Log::inline(42, 1024).unwrap();
        let foreign = server.client.logs("PerishLab", "wharf", query).unwrap_err();
        assert_eq!(
            (foreign.outcome, foreign.cause),
            (Outcome::Unknown, Cause::Shape)
        );
        let gone = server.client.logs("PerishLab", "wharf", query).unwrap_err();
        assert_eq!((gone.outcome, gone.cause), (Outcome::Refused, Cause::Http));
        let sent = server.thread.join().unwrap();
        assert!(sent[0].starts_with("GET /repos/PerishLab/wharf/actions/jobs/42/logs "));
    }

    #[test]
    fn delivery() {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let (url, thread) = local("200 OK", "step ok\n");
        let mut reply = agent.get(&url).call().unwrap();
        let receipt = Log::inline(9, 64)
            .unwrap()
            .read(&mut reply, Evidence::default())
            .unwrap();
        assert_eq!(receipt.value.id, 9);
        assert_eq!(receipt.value.size, 8);
        assert_eq!(receipt.value.text, "step ok\n");
        let headers = thread.join().unwrap().to_ascii_lowercase();
        assert!(!headers.contains("authorization:"));

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("job.log");
        let (url, thread) = local("200 OK", "raw bytes");
        let mut reply = agent.get(&url).call().unwrap();
        let staged = stage(&target).unwrap();
        let saved = Log::file(9, 64)
            .unwrap()
            .write(&mut reply, staged, &target, Evidence::default())
            .unwrap();
        assert_eq!(saved.value.size, 9);
        assert_eq!(std::fs::read(&target).unwrap(), b"raw bytes");
        thread.join().unwrap();

        let (url, thread) = local("200 OK", "too long");
        let mut reply = agent.get(&url).call().unwrap();
        let error = Log::inline(9, 4)
            .unwrap()
            .read(&mut reply, Evidence::default())
            .unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
        thread.join().unwrap();
    }
}
