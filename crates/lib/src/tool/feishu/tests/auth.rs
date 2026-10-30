use super::*;
use serde_json::json;

#[test]
fn issuance() {
    let held = credential(
        &json!({"token_type":"Bearer","access_token":"TOKEN_FIXTURE","expires_in":7200}),
        100,
    )
    .unwrap();
    assert_eq!(held.expires, 7300);
    for value in [
        json!({"token_type":"DPoP","access_token":"SECRET_FIXTURE","expires_in":7200}),
        json!({"token_type":"Bearer","access_token":"SECRET_FIXTURE","expires_in":-1}),
        json!({"token_type":"Bearer","access_token":"SECRET_FIXTURE","expires_in":59}),
        json!({"token_type":"Bearer","access_token":"SECRET_FIXTURE","expires_in":99999999}),
        json!({"token_type":"Bearer","access_token":"SECRET\nFIXTURE","expires_in":7200}),
        json!({}),
    ] {
        let error = credential(&value, 100).err().unwrap();
        assert_eq!(error.outcome, super::super::Outcome::Unknown);
        assert!(!format!("{error:?}").contains("SECRET"));
    }
}
