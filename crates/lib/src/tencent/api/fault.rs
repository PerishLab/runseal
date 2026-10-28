use serde::Serialize;
use serde_json::Value;
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
    pub(in crate::tencent) fn new(class: Class, detail: impl Into<String>) -> Self {
        Self {
            class,
            code: None,
            request: None,
            status: None,
            detail: detail.into(),
        }
    }

    pub(in crate::tencent) fn capture(error: anyhow::Error, class: Class) -> Self {
        error.downcast_ref::<Self>().cloned().unwrap_or_else(|| {
            Self::new(
                class,
                "Tencent operation refused; no private data is emitted",
            )
        })
    }

    pub(super) fn provider(value: &Value) -> Self {
        let code = value["Error"]["Code"].as_str().unwrap_or_default();
        let (class, reason) = match code {
            "UnauthorizedOperation" | "AuthFailure.UnauthorizedOperation" => {
                (Class::Permission, "required permission was refused")
            }
            "AuthFailure.TokenExpired" | "AuthFailure.SignatureExpire" => {
                (Class::Expired, "credentials expired")
            }
            "AuthFailure.SignatureFailure" => (Class::Authentication, "signature was refused"),
            "AuthFailure.TokenFailure"
            | "AuthFailure.SecretIdNotFound"
            | "AuthFailure.InvalidSecretId" => (Class::Authentication, "credentials were refused"),
            "RequestLimitExceeded" => (Class::Limit, "request limit was reached"),
            "UnsupportedOperation" => (Class::Unsupported, "operation is unsupported"),
            _ => (Class::Provider, "operation was refused"),
        };
        let mut failure = Self::new(
            class,
            format!("Tencent {reason}; no provider message or raw response is emitted"),
        );
        if class != Class::Provider {
            failure.code = Some(code.into());
        }
        failure.request = value["RequestId"]
            .as_str()
            .filter(|id| {
                !id.is_empty()
                    && id.len() <= 128
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
            .map(str::to_owned);
        failure
    }

    pub(super) fn status(status: u16) -> Self {
        let mut failure = Self::new(
            Class::Response,
            format!(
                "Tencent HTTP response refused ({status}); the remote operation may have completed; no automatic retry"
            ),
        );
        failure.status = Some(status);
        failure
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for Failure {}
