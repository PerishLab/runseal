use runseal::{
    custody::Private,
    feishu::{Class, Client, Operation, Outcome},
    tool,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    state: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        #[cfg(windows)]
        let root = tempfile::tempdir_in(std::env::var_os("ProgramData").unwrap()).unwrap();
        #[cfg(unix)]
        let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let folder = root.path().join("private");
        Private::current().folder(&folder).unwrap();
        Self {
            state: folder.join("state.json"),
            _root: root,
        }
    }

    fn write(&self, value: Value) {
        Private::current()
            .create(&self.state, &serde_json::to_vec(&value).unwrap())
            .unwrap();
    }

    fn call(&self, words: &[&str]) -> anyhow::Result<tool::Reply> {
        tool::call(
            "feishu",
            &words
                .iter()
                .map(|word| word.to_string())
                .collect::<Vec<_>>(),
            &BTreeMap::from([
                ("FEISHU_STATE_FILE".into(), self.state.display().to_string()),
                ("FEISHU_APP_ID".into(), "cli_fixture".into()),
            ]),
        )
    }
}

#[test]
fn import() {
    let fixture = Fixture::new();
    let source = fixture.state.with_file_name("credentials.json");
    Private::current()
        .create(
            &source,
            br#"{"app_id":"cli_fixture","app_secret":"SECRET_FIXTURE"}"#,
        )
        .unwrap();
    let status = Client::import(&fixture.state, "cli_fixture", &source).unwrap();
    assert!(!status.valid);
    assert!(
        !serde_json::to_string(&status)
            .unwrap()
            .contains("SECRET_FIXTURE")
    );
    assert!(Client::import(&fixture.state, "cli_fixture", &source).is_err());
    let output = fixture.call(&["auth", "status"]).unwrap();
    assert_eq!(output.value["app"], "cli_fixture");
    assert!(!format!("{output:?}").contains("SECRET_FIXTURE"));
    let client = Client::open(&fixture.state, "cli_fixture").unwrap();
    assert!(Client::open(&fixture.state, "cli_fixture").is_err());
    assert_eq!(
        client
            .message(&Operation::Get {
                message: "om_fixture".into()
            })
            .unwrap_err()
            .class,
        Class::Authentication
    );
}

#[test]
fn binding() {
    for (credential, identity, class) in [
        (Value::Null, Value::Null, Class::Authentication),
        (
            json!({"token":"TOKEN_FIXTURE","expires":1}),
            Value::Null,
            Class::Expired,
        ),
        (
            json!({"token":"TOKEN_FIXTURE","expires":4102444800_i64}),
            Value::Null,
            Class::Binding,
        ),
        (
            json!({"token":"TOKEN_FIXTURE","expires":4102444800_i64}),
            json!({"app":"cli_other","bot":"ou_bot","observed":100,"stamp":"wrong"}),
            Class::Binding,
        ),
        (
            json!({"token":"TOKEN_FIXTURE","expires":4102444800_i64}),
            json!({"app":"cli_fixture","bot":"ou_bot","observed":100,"stamp":"wrong"}),
            Class::Binding,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":credential,"identity":identity}));
        let client = Client::open(&fixture.state, "cli_fixture").unwrap();
        let error = client
            .message(&Operation::Get {
                message: "om_fixture".into(),
            })
            .unwrap_err();
        assert_eq!(error.class, class);
        assert_eq!(error.outcome, Outcome::Unsent);
        assert!(!format!("{error:?} {error}").contains("FIXTURE"));
        drop(client);
        assert!(Client::open(&fixture.state, "cli_foreign").is_err());
    }
}

#[test]
fn status() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE",
        "credential":{"token":"TOKEN_FIXTURE","expires":4102444800_i64},
        "identity":{"app":"cli_fixture","bot":"ou_bot","observed":100,"stamp":format!("{:x}", Sha256::digest(b"TOKEN_FIXTURE"))}}));
    let output = fixture.call(&["auth", "status"]).unwrap();
    assert_eq!(output.value["valid"], true);
    assert_eq!(output.value["identity"]["bot"], "ou_bot");
    assert!(!format!("{output:?}").contains("FIXTURE"));
}

#[cfg(unix)]
#[test]
fn custody() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    std::fs::set_permissions(&fixture.state, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Client::open(&fixture.state, "cli_fixture").is_err());
    std::fs::set_permissions(&fixture.state, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = fixture.state.with_file_name("symlink.json");
    symlink(&fixture.state, &link).unwrap();
    assert!(Client::open(&link, "cli_fixture").is_err());
}
