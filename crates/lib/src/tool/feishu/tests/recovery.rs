use super::*;
use crate::custody::Private;
use serde_json::json;

#[test]
fn retention() {
    let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let folder = root.path().join("private");
    Private::current().folder(&folder).unwrap();
    let path = folder.join("state.json");
    let source = folder.join("credentials.json");
    Private::current()
        .create(
            &source,
            br#"{"app_id":"cli_fixture","app_secret":"SECRET_FIXTURE"}"#,
        )
        .unwrap();
    Client::import(&path, "cli_fixture", &source).unwrap();
    let mut client = Client::user(&path, "cli_fixture", "ou_user").unwrap();
    let mut user = auth::User::new("ou_user");
    user.uncertain = true;
    client.state.user = Some(user);
    let returned = auth::Return {
        body: json!({"token_type":"Bearer","access_token":"TOKEN_FIXTURE","expires_in":7200})
            .to_string(),
        started: 100,
        request: Some("SAFE-REQUEST".into()),
        kind: auth::Kind::Exchange,
    };
    let error = client.retain(returned).unwrap_err();
    assert_eq!(error.field, Some(super::super::super::Field::Refresh));
    assert_eq!(error.outcome, Outcome::Unknown);
    assert_eq!(error.request.as_deref(), Some("SAFE-REQUEST"));
    drop(client);
    let mut client = Client::user(&path, "cli_fixture", "ou_user").unwrap();
    let status = client.status();
    let user = status.user.unwrap();
    assert!(user.retained && user.uncertain);
    assert_eq!(user.response.unwrap().token, Some(13));
    assert!(
        !serde_json::to_string(&client.status())
            .unwrap()
            .contains("FIXTURE")
    );
    assert_eq!(
        client.recover().unwrap_err().field,
        Some(super::super::super::Field::Refresh)
    );
    assert_eq!(client.renew().unwrap_err().outcome, Outcome::Unsent);
}
