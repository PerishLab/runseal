use super::Class;

pub(super) fn classify(code: &str) -> Option<(Class, &'static str)> {
    Some(match code {
        "InvalidParameter.EntityFilterError"
        | "InvalidParameter.PolicyIdError"
        | "InvalidParameter.PolicyIdNotExist"
        | "ResourceNotFound.PolicyIdNotFound" => (
            Class::Input,
            "CAM policy scope was refused; qualify exact identity and prior receipts before interpreting absence",
        ),
        "UnauthorizedOperation.NoPermission" => {
            (Class::Permission, "required permission was refused")
        }
        "UnauthorizedOperation.MFAExpired" | "UnauthorizedOperation.MFANotFound" => (
            Class::Authentication,
            "an explicit human MFA ceremony is required",
        ),
        "InvalidParameterValue.DuplicateParameterValue"
        | "InvalidParameterValue.KeyPairPublicKeyDuplicated"
        | "InvalidParameterValue.KeyPairPublicKeyMalformed"
        | "InvalidParameterValue.InvalidKeyPairNameEmpty"
        | "InvalidParameterValue.InvalidKeyPairNameTooLong"
        | "InvalidParameterValue.KeyPairIdMalformed"
        | "ResourceNotFound.KeyIdNotFound"
        | "ResourceNotFound.InstanceIdNotFound"
        | "ResourceNotFound.InstanceNotFound" => (
            Class::Input,
            "Lighthouse input or exact scope refused; this alone proves no absence",
        ),
        "ResourceInUse.KeyPairInUse"
        | "UnsupportedOperation.KeyPairBindDuplicate"
        | "UnsupportedOperation.LatestOperationUnfinished" => (
            Class::Binding,
            "Lighthouse reference or operation conflict; inspect exact intent before recovery",
        ),
        "LimitExceeded.KeyPairLimitExceeded" => (Class::Limit, "public-key quota was refused"),
        "UnsupportedOperation.KeyPairBindToBlueprints"
        | "UnsupportedOperation.InvalidInstanceState"
        | "UnsupportedOperation.InstanceDisplayAreaNotSupportOperation"
        | "UnsupportedOperation.WindowsNotAllowToAssociateKeyPair"
        | "UnsupportedOperation.WindowsNotSupportKeyPair" => (
            Class::Unsupported,
            "Lighthouse operation is unsupported for this resource",
        ),
        _ => return None,
    })
}

#[test]
fn certainty() {
    use super::{Error, Failure, Outcome};
    for code in [
        "UnauthorizedOperation.NoPermission",
        "UnauthorizedOperation.MFAExpired",
        "ResourceNotFound.KeyIdNotFound",
        "ResourceInUse.KeyPairInUse",
    ] {
        let fault = Failure::provider(
            &serde_json::json!({"Error":{"Code":code,"Message":"PRIVATE_MARKER"},"RequestId":"fixture-id"}),
        );
        assert_eq!(fault.code.as_deref(), Some(code));
        assert_eq!(Error::remote(fault.clone()).outcome, Outcome::Rejected);
        assert!(!format!("{fault:?}").contains("PRIVATE_MARKER"));
    }
    for code in [
        "FailedOperation.ImportKeyPairFailed",
        "FailedOperation.DeleteKeyPairFailed",
        "FailedOperation.InstanceOperationFailed",
        "InternalError.InvalidCommandNotFound",
        "PRIVATE_MARKER",
    ] {
        let fault = Failure::provider(
            &serde_json::json!({"Error":{"Code":code,"Message":"PRIVATE_MARKER"},"RequestId":"fixture-id"}),
        );
        assert!(fault.code.is_none());
        assert_eq!(Error::remote(fault.clone()).outcome, Outcome::Unknown);
        assert!(!format!("{fault:?}").contains("PRIVATE_MARKER"));
    }
}
