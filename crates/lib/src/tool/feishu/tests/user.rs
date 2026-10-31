use super::*;
use serde_json::json;

#[test]
fn grant() {
    let mut value = json!({"token_type":"Bearer","access_token":"TOKEN_FIXTURE","expires_in":7200,
        "refresh_token":"REFRESH_FIXTURE","refresh_token_expires_in":604800,
        "scope":"offline_access im:message.send_as_user"});
    let original = grant::decode(&value, 100, None).unwrap();
    value.as_object_mut().unwrap().remove("refresh_token");
    value
        .as_object_mut()
        .unwrap()
        .remove("refresh_token_expires_in");
    value.as_object_mut().unwrap().remove("scope");
    let refreshed = grant::decode(&value, 200, Some(&original)).unwrap();
    assert_eq!(refreshed.expires, original.expires);
    assert_eq!(refreshed.refresh, original.refresh);
    assert_eq!(refreshed.scopes, original.scopes);
    assert!(grant::decode(&value, 200, None).is_err());
    for field in ["access_token", "expires_in", "token_type"] {
        let mut invalid = value.clone();
        invalid.as_object_mut().unwrap().remove(field);
        let error = grant::decode(&invalid, 200, Some(&original)).err().unwrap();
        assert!(!format!("{error:?}").contains("FIXTURE"));
        assert_eq!(error.outcome, super::super::Outcome::Unknown);
    }
    value["token_type"] = json!("DPoP");
    assert!(grant::decode(&value, 200, Some(&original)).is_err());
}

#[test]
fn challenge() {
    let mut value = json!({"device_code":"DEVICE_SECRET","user_code":"ABCD-EFGH",
        "verification_uri":"https://accounts.feishu.cn/oauth/device",
        "verification_uri_complete":"https://evil.invalid/SECRET", "expires_in":240,"interval":5});
    let held = device::decode(&value, 100, vec!["offline_access".into()]).unwrap();
    let public = serde_json::to_string(&held.challenge).unwrap();
    assert!(!public.contains("SECRET"));
    assert!(!public.contains("evil"));
    assert_eq!(held.next, 105);
    for uri in [
        "https://evil.invalid/",
        "http://accounts.feishu.cn/",
        "https://accounts.feishu.cn/?secret=x",
        "https://user@accounts.feishu.cn/",
        "https://accounts.feishu.cn/#secret",
    ] {
        value["verification_uri"] = json!(uri);
        assert!(device::decode(&value, 100, vec![]).is_err());
    }
}
