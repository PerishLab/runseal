use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    Input,
    Custody,
    Binding,
    Authentication,
    Permission,
    Expired,
    Network,
    Limit,
    Response,
    Unsupported,
    Provider,
}

#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub class: Class,
    pub code: Option<String>,
    pub request: Option<String>,
    pub status: Option<u16>,
    detail: String,
}

impl Failure {
    pub(super) fn new(class: Class, detail: impl Into<String>) -> Self {
        Self {
            class,
            code: None,
            request: None,
            status: None,
            detail: detail.into(),
        }
    }

    pub(super) fn capture(error: anyhow::Error, class: Class) -> Self {
        error.downcast_ref::<Self>().cloned().unwrap_or_else(|| {
            Self::new(
                class,
                "Alibaba operation refused; no private data is emitted",
            )
        })
    }

    pub(super) fn status(status: u16) -> Self {
        let mut failure = Self::new(
            Class::Response,
            format!(
                "Alibaba HTTP response refused ({status}); the remote operation may have completed; no automatic retry"
            ),
        );
        failure.status = Some(status);
        failure
    }

    pub(super) fn network(detail: &'static str) -> Self {
        Self::new(Class::Network, detail)
    }

    pub(super) fn request(mut self, id: Option<&str>) -> Self {
        self.request = id
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 128
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
            .map(str::to_owned);
        self
    }

    pub(super) fn oauth(status: u16, code: Option<&str>) -> Self {
        let (class, reason) = match code {
            Some("invalid_grant") => (
                Class::Authentication,
                "the authorization grant was refused or already used; explicitly begin a new authorization",
            ),
            Some("invalid_client") => (
                Class::Authentication,
                "the application identity was refused",
            ),
            Some("invalid_request") => (Class::Input, "the token request was refused"),
            Some("unauthorized_client") => {
                (Class::Permission, "the application may not use this grant")
            }
            Some("unsupported_grant_type") => (Class::Unsupported, "the grant type is unsupported"),
            Some("access_denied") => (
                Class::Permission,
                "authorization was denied at the provider",
            ),
            Some("expired_token" | "invalid_token") => (
                Class::Expired,
                "the token was expired or invalid; explicitly begin a new authorization",
            ),
            Some("insufficient_scope") => (Class::Permission, "required scope was not granted"),
            _ => (Class::Provider, "the token endpoint refused the request"),
        };
        let mut failure = Self::new(
            class,
            format!("Alibaba {reason}; no provider prose or token material is emitted"),
        );
        failure.status = Some(status);
        failure.code = code.filter(|_| class != Class::Provider).map(str::to_owned);
        failure
    }

    pub(super) fn acs(status: u16, code: Option<&str>, request: Option<&str>) -> Self {
        let (class, reason) = match code {
            Some("InvalidAccessKeyId.NotFound" | "InvalidAccessKeyId.Inactive") => (
                Class::Authentication,
                "the temporary credential was refused",
            ),
            Some("SignatureDoesNotMatch") => {
                (Class::Authentication, "the request signature was refused")
            }
            Some("InvalidSecurityToken.Malformed" | "InvalidSecurityToken.Expired") => (
                Class::Expired,
                "the temporary security token was expired or malformed",
            ),
            Some("Forbidden.RAM" | "Forbidden" | "NoPermission") => {
                (Class::Permission, "required permission was refused")
            }
            Some("Throttling" | "Throttling.User" | "Throttling.Api") => {
                (Class::Limit, "the request limit was reached")
            }
            Some("InvalidParameter" | "MissingParameter" | "InvalidVersion" | "InvalidAction") => {
                (Class::Input, "the request scope was refused")
            }
            Some("ServiceUnavailable" | "InternalError") => (
                Class::Unsupported,
                "the provider could not serve the request",
            ),
            _ => (Class::Provider, "the provider refused the request"),
        };
        let mut failure = Self::new(
            class,
            format!("Alibaba {reason}; no provider prose or raw response is emitted"),
        );
        failure.status = Some(status);
        failure.code = code.filter(|_| class != Class::Provider).map(str::to_owned);
        failure.request = request.map(str::to_owned).filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
        failure
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)?;
        if let Some(code) = &self.code {
            write!(formatter, "; code {code}")?;
        }
        if let Some(request) = &self.request {
            write!(formatter, "; request {request}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Failure {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Unsent,
    Rejected,
    Unknown,
    Accepted,
    Observed,
}

impl fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Unsent => "unsent",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
            Self::Accepted => "accepted",
            Self::Observed => "observed",
        };
        formatter.write_str(value)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Error {
    pub fault: Failure,
    pub outcome: Outcome,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}; outcome {}; no automatic replay",
            self.fault, self.outcome
        )
    }
}

impl std::error::Error for Error {}

impl Error {
    pub(super) fn remote(fault: Failure) -> Self {
        if fault.status.is_some_and(|status| status >= 500) {
            return Self {
                fault,
                outcome: Outcome::Unknown,
            };
        }
        let refused = matches!(
            fault.class,
            Class::Permission
                | Class::Expired
                | Class::Authentication
                | Class::Input
                | Class::Limit
                | Class::Unsupported
        );
        Self {
            fault,
            outcome: if refused {
                Outcome::Rejected
            } else {
                Outcome::Unknown
            },
        }
    }
}

#[test]
fn classified() {
    for (code, class) in [
        ("invalid_grant", Class::Authentication),
        ("insufficient_scope", Class::Permission),
        ("expired_token", Class::Expired),
        ("unrecognized_fixture", Class::Provider),
    ] {
        let fault = Failure::oauth(400, Some(code));
        assert_eq!(fault.class, class);
        assert!(!fault.to_string().contains("description"));
    }
    for (code, class) in [
        ("Forbidden.RAM", Class::Permission),
        ("InvalidAccessKeyId.NotFound", Class::Authentication),
        ("Throttling", Class::Limit),
        ("UnknownCode", Class::Provider),
    ] {
        let fault = Failure::acs(403, Some(code), Some("fixture-request-id"));
        assert_eq!(fault.class, class);
        assert_eq!(
            fault.code.as_deref(),
            Some(code).filter(|_| class != Class::Provider)
        );
    }
    assert_eq!(
        Error::remote(Failure::new(Class::Response, "fixture")).outcome,
        Outcome::Unknown
    );
    for status in [500, 502, 503] {
        let fault = Failure::acs(status, Some("Throttling"), Some("fixture-request-id"));
        assert_eq!(Error::remote(fault).outcome, Outcome::Unknown);
        let fault = Failure::oauth(status, Some("invalid_grant"));
        assert_eq!(Error::remote(fault).outcome, Outcome::Unknown);
    }
    let fault = Failure::acs(403, Some("Forbidden.RAM"), None);
    assert_eq!(Error::remote(fault).outcome, Outcome::Rejected);
}

#[test]
fn redaction() {
    let fault = Failure::acs(403, Some("PRIVATE_MARKER"), Some("PRIVATE MARKER"));
    let text = fault.to_string();
    assert!(!text.contains("PRIVATE_MARKER"));
    assert!(!text.contains("PRIVATE MARKER"));
    assert!(!text.contains('_'));
    let network = Error::remote(Failure::network(
        "Alibaba transport produced no observed response",
    ));
    assert_eq!(network.outcome, Outcome::Unknown);
    assert!(network.to_string().contains("unknown"));
}
