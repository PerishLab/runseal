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
            "UnsupportedOperation.VersionMismatch" => (
                Class::Binding,
                "security group policy version changed; read the complete policy set before preparing a new intent",
            ),
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
            "FailedOperation.PolicyFull" | "InvalidParameter.AttachmentFull" => (
                Class::Limit,
                "CAM policy or attachment capacity was reached",
            ),
            "FailedOperation.PolicyNameInUse" => {
                (Class::Input, "CAM policy name is already in use")
            }
            "InvalidParameter.ActionError"
            | "InvalidParameter.ActionMiss"
            | "InvalidParameter.ActionNotExist"
            | "InvalidParameter.ActionServiceNotExist"
            | "InvalidParameter.ConditionContentError"
            | "InvalidParameter.ConditionError"
            | "InvalidParameter.ConditionTypeError"
            | "InvalidParameter.DescriptionLengthOverlimit"
            | "InvalidParameter.EffectError"
            | "InvalidParameter.NotSupportProduct"
            | "InvalidParameter.ParamError"
            | "InvalidParameter.PolicyDocumentError"
            | "InvalidParameter.PolicyDocumentLengthOverLimit"
            | "InvalidParameter.PolicyNameError"
            | "InvalidParameter.PrincipalError"
            | "InvalidParameter.PrincipalQcsError"
            | "InvalidParameter.PrincipalQcsNotExist"
            | "InvalidParameter.PrincipalServiceNotExist"
            | "InvalidParameter.ResourceContentError"
            | "InvalidParameter.ResourceError"
            | "InvalidParameter.ResourceProjectError"
            | "InvalidParameter.ResourceQcsError"
            | "InvalidParameter.ResourceRegionError"
            | "InvalidParameter.ResourceServiceNotExist"
            | "InvalidParameter.ResourceUinError"
            | "InvalidParameter.StatementError"
            | "InvalidParameter.UserNotExist"
            | "InvalidParameter.VersionError"
            | "ResourceNotFound.GroupNotExist"
            | "ResourceNotFound.NotFound"
            | "ResourceNotFound.UserNotExist" => {
                (Class::Input, "CAM input or referenced resource was refused")
            }
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

#[test]
fn diagnostic() {
    for (code, class) in [
        ("InvalidParameter.ActionNotExist", Class::Input),
        ("FailedOperation.PolicyFull", Class::Limit),
        ("FailedOperation.PolicyNameInUse", Class::Input),
    ] {
        let failure = Failure::provider(
            &serde_json::json!({"Error":{"Code":code,"Message":"SECRET_FIXTURE"},"RequestId":"fixture-id"}),
        );
        assert_eq!(failure.class, class);
        assert_eq!(failure.code.as_deref(), Some(code));
        let output = failure.to_string();
        assert!(output.contains(code));
        assert!(output.contains("fixture-id"));
        assert!(!output.contains("SECRET_FIXTURE"));
    }
}
