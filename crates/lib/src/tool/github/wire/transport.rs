use crate::github::{Cause, Error, Evidence, Outcome};
use crate::http::wire::{Payload, Request};
use base64::{Engine, engine::general_purpose::STANDARD};
use rustls::{RootCertStore, pki_types::CertificateDer};
use std::{sync::Arc, time::Duration};
use ureq::tls::{Certificate, RootCerts, TlsConfig};
use zeroize::Zeroizing;

pub(in crate::tool::github) const LIMIT: usize = 64 * 1024;

pub(super) struct Transport {
    agent: ureq::Agent,
    tls: TlsConfig,
    seconds: u64,
}

impl Transport {
    pub(super) fn new(seconds: u64) -> Self {
        let tls = TlsConfig::default();
        Self {
            agent: agent(seconds, &tls),
            tls,
            seconds,
        }
    }

    pub(super) fn roots(&mut self, pem: &[u8]) -> Result<(), Error> {
        let roots = certificates(pem).ok_or_else(|| Error::local(Cause::Input))?;
        self.tls = TlsConfig::builder()
            .root_certs(RootCerts::Specific(roots))
            .build();
        self.timeout(self.seconds);
        Ok(())
    }

    pub(super) fn timeout(&mut self, seconds: u64) {
        self.agent = agent(seconds, &self.tls);
        self.seconds = seconds;
    }

    pub(super) fn request(
        &self,
        request: Request<'_>,
        attempts: u8,
        limit: u64,
    ) -> Result<Payload, Error> {
        let mut outbound = ureq::http::Request::builder()
            .method(request.method)
            .uri(request.url);
        for &(name, value) in request.headers {
            outbound = outbound.header(name, value);
        }
        let mut reply = match request.body {
            Some(body) => outbound
                .body(body)
                .ok()
                .and_then(|request| self.agent.run(request).ok()),
            None => outbound
                .body(())
                .ok()
                .and_then(|request| self.agent.run(request).ok()),
        }
        .ok_or_else(|| failure(Cause::Network, attempts, None))?;
        let status = reply.status().as_u16();
        let headers = reply
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                Some((
                    name.as_str().to_owned(),
                    Zeroizing::new(value.to_str().ok()?.to_owned()),
                ))
            })
            .collect();
        let body = reply
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|_| failure(Cause::Read, attempts, Some(status)))?;
        Ok(Payload {
            status,
            headers,
            body: Zeroizing::new(body),
        })
    }
}

fn agent(seconds: u64, tls: &TlsConfig) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(seconds)))
        .timeout_connect(Some(Duration::from_secs(seconds.min(3))))
        .tls_config(tls.clone())
        .build()
        .into()
}

fn failure(cause: Cause, attempts: u8, status: Option<u16>) -> Error {
    Error {
        outcome: Outcome::Unknown,
        cause,
        evidence: Evidence {
            attempts,
            status,
            ..Evidence::default()
        },
    }
}

fn certificates(pem: &[u8]) -> Option<Arc<Vec<Certificate<'static>>>> {
    if pem.len() > LIMIT {
        return None;
    }
    let mut remaining = std::str::from_utf8(pem).ok()?.trim_ascii();
    let mut roots = Vec::new();
    let mut store = RootCertStore::empty();
    while !remaining.is_empty() {
        let body = remaining.strip_prefix("-----BEGIN CERTIFICATE-----")?;
        let body = body
            .strip_prefix("\r\n")
            .or_else(|| body.strip_prefix('\n'))?;
        let (encoded, tail) = body.split_once("\n-----END CERTIFICATE-----")?;
        let encoded: Vec<_> = encoded
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        let der = STANDARD.decode(encoded).ok()?;
        store.add(CertificateDer::from(der.as_slice())).ok()?;
        roots.push(Certificate::from_der(&der).to_owned());
        if !tail.is_empty() && !tail.starts_with(['\r', '\n']) {
            return None;
        }
        remaining = tail.trim_ascii();
    }
    (!roots.is_empty()).then(|| Arc::new(roots))
}
