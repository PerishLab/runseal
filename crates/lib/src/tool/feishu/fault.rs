use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    Input,
    Custody,
    Binding,
    Authentication,
    Expired,
    Permission,
    Limit,
    Network,
    Response,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Unsent,
    Rejected,
    Unknown,
    Accepted,
    Observed,
}

#[derive(Clone, Debug, Serialize)]
pub struct Error {
    pub class: Class,
    pub outcome: Outcome,
    pub code: Option<i64>,
    pub status: Option<u16>,
    pub request: Option<String>,
    detail: &'static str,
}

impl Error {
    pub(super) fn new(class: Class, detail: &'static str) -> Self {
        Self {
            class,
            outcome: Outcome::Unsent,
            code: None,
            status: None,
            request: None,
            detail,
        }
    }

    pub(super) fn unknown(class: Class) -> Self {
        let mut error = Self::new(
            class,
            "Feishu result unknown; inspect the exact resource before recovery; no automatic replay",
        );
        error.outcome = Outcome::Unknown;
        error
    }

    pub(super) fn provider(code: Option<i64>) -> Self {
        let class = match code {
            Some(20002 | 20048 | 20130 | 20140 | 99991661 | 99991663 | 99991665) => {
                Class::Authentication
            }
            Some(99991677) => Class::Expired,
            Some(20009 | 20068 | 20069 | 99991662 | 99991672) => Class::Permission,
            Some(20001 | 20036 | 20063 | 20067 | 20070) => Class::Input,
            Some(99991400) => Class::Limit,
            _ => Class::Response,
        };
        let mut error = Self::unknown(class);
        if class != Class::Response {
            error.outcome = Outcome::Rejected;
            error.detail = "Feishu refused the operation; inspect permissions, identity or input; no automatic replay";
        }
        error.code = code.filter(|code| (0..=999_999_999).contains(code));
        error
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{}; outcome {:?}", self.detail, self.outcome)?;
        if let Some(code) = self.code {
            write!(out, "; code {code}")?;
        }
        if let Some(request) = &self.request {
            write!(out, "; request {request}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

pub(super) fn input() -> Error {
    Error::new(
        Class::Input,
        "Feishu input refused; use the typed command help",
    )
}

pub(super) fn response() -> Error {
    Error::unknown(Class::Response)
}
