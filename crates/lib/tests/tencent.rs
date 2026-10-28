use base64::{Engine, engine::general_purpose::STANDARD};
use runseal::{custody::Private, tool};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    state: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        #[cfg(windows)]
        let root =
            tempfile::tempdir_in(std::env::var_os("ProgramData").expect("ProgramData")).unwrap();
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

    fn call(&self, words: &[&str]) -> anyhow::Result<tool::Reply> {
        let vars = BTreeMap::from([(
            "TENCENTCLOUD_STATE_FILE".into(),
            self.state.display().to_string(),
        )]);
        tool::call(
            "tencent",
            &words
                .iter()
                .map(|word| word.to_string())
                .collect::<Vec<_>>(),
            &vars,
        )
    }

    fn write(&self, value: &Value) {
        Private::current()
            .create(&self.state, &serde_json::to_vec(value).unwrap())
            .unwrap();
    }
}

#[test]
fn challenge() {
    let held = Fixture::new();
    let first = held.call(&["auth", "begin"]).unwrap();
    let second = held.call(&["auth", "begin"]).unwrap();
    assert_eq!(first.value, second.value);
    let url = url::Url::parse(first.value["authorizationUrl"].as_str().unwrap()).unwrap();
    assert_eq!(url.host_str(), Some("cloud.tencent.com"));
    let nonce = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    assert_eq!(nonce.len(), 64);
    let private = Private::current().read(&held.state, 16384).unwrap();
    let stored: Value = serde_json::from_slice(&private).unwrap();
    assert_eq!(stored["intent"]["state"], nonce);
    assert!(
        held.call(&["auth", "status"]).unwrap().value["pending"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn binding() {
    let held = Fixture::new();
    held.call(&["auth", "begin"]).unwrap();
    let response = held.state.with_file_name("response.txt");
    let encoded = STANDARD.encode(json!({"state":"wrong","site":"cn","accessToken":"SENSITIVE_AUTHORIZATION_FIXTURE","refreshToken":"SENSITIVE_AUTHORIZATION_FIXTURE","openId":"subject","expiresAt":9999999999u64}).to_string());
    Private::current()
        .create(&response, encoded.as_bytes())
        .unwrap();
    let error = held
        .call(&[
            "auth",
            "complete",
            "--response-file",
            response.to_str().unwrap(),
        ])
        .err()
        .unwrap();
    assert!(error.to_string().contains("state or site"));
    assert!(!format!("{error:#?} {error:#}").contains("SENSITIVE_AUTHORIZATION_FIXTURE"));
    assert!(
        held.call(&["auth", "status"]).unwrap().value["pending"]
            .as_bool()
            .unwrap()
    );
    assert!(response.exists());
}

#[test]
fn expired() {
    let held = Fixture::new();
    held.write(&json!({"version":1,"intent":null,"oauth":null,"credential":{"id":"ID","key":"SENSITIVE_CREDENTIAL_FIXTURE","token":null,"expires":1},"identity":null}));
    let receipt = held.call(&["auth", "status"]).unwrap();
    assert!(!format!("{receipt:?}").contains("SENSITIVE_CREDENTIAL_FIXTURE"));
    let error = held.call(&["identity"]).err().unwrap();
    assert!(error.to_string().contains("expired"));
    assert!(!format!("{error:#?}").contains("SENSITIVE_CREDENTIAL_FIXTURE"));
}

#[test]
fn authority() {
    let held = Fixture::new();
    held.write(&json!({"version":1,"intent":null,"oauth":null,"credential":{"id":"ID","key":"KEY","token":null,"expires":1},"identity":null}));
    let error = held
        .call(&[
            "--account",
            "123",
            "cam",
            "user",
            "create",
            "--name",
            "fixture",
        ])
        .err()
        .unwrap();
    assert!(error.to_string().contains("verify Tencent caller identity"));
    let error = held
        .call(&["cam", "user", "create", "--name", "fixture"])
        .err()
        .unwrap();
    assert!(error.to_string().contains("mutations require"));
}

#[test]
fn logout() {
    let held = Fixture::new();
    held.write(&json!({"version":1,"intent":null,"oauth":{"subject":"subject","access":"SENSITIVE_ACCESS_FIXTURE","refresh":"SENSITIVE_REFRESH_FIXTURE","expires":9999999999u64,"site":"cn"},"credential":null,"identity":null}));
    let result = held.call(&["auth", "logout"]).unwrap();
    assert_eq!(result.value["localLogout"], true);
    assert!(
        result.value["remoteRevocation"]
            .as_str()
            .unwrap()
            .contains("not performed")
    );
    let stored = Private::current().read(&held.state, 16384).unwrap();
    assert!(!String::from_utf8_lossy(&stored).contains("SENSITIVE"));
    held.call(&["auth", "begin"]).unwrap();
}

#[test]
fn exclusive() {
    let held = Fixture::new();
    held.call(&["auth", "begin"]).unwrap();
    let lock = Private::current()
        .lock(&held.state.with_extension("lock"))
        .unwrap();
    let error = held.call(&["auth", "status"]).err().unwrap();
    assert!(error.to_string().contains("already in use"));
    drop(lock);
    held.call(&["auth", "status"]).unwrap();
}

#[test]
fn destination() {
    let held = Fixture::new();
    let file = Private::current().reserve(&held.state).unwrap();
    assert!(Private::current().reserve(&held.state).is_err());
    drop(file);
    let link = held.state.with_file_name("linked.json");
    std::fs::hard_link(&held.state, &link).unwrap();
    assert!(Private::current().read(&held.state, 16384).is_err());
    assert!(Private::current().read(&link, 16384).is_err());
}

#[test]
fn escape() {
    let held = Fixture::new();
    for words in [
        vec!["--url", "http://127.0.0.1", "identity"],
        vec!["cam", "request", "CreateAnything"],
        vec!["--secret-key", "SENSITIVE_ARG_FIXTURE", "identity"],
    ] {
        let error = held.call(&words).err().unwrap();
        assert!(!format!("{error:#?}").contains("SENSITIVE_ARG_FIXTURE"));
        assert!(error.to_string().contains("invalid @tencent command"));
    }
    assert!(!held.state.exists());
}

#[test]
fn pending() {
    let held = Fixture::new();
    held.write(&json!({"version":1,"intent":{"state":"fixture","deadline":1,"consumed":false},"oauth":null,"credential":null,"identity":null}));
    let response = held.state.with_file_name("absent.txt");
    let error = held
        .call(&[
            "auth",
            "complete",
            "--response-file",
            response.to_str().unwrap(),
        ])
        .err()
        .unwrap();
    assert!(error.to_string().contains("expired or consumed"));
    assert!(
        !held.call(&["auth", "status"]).unwrap().value["pending"]
            .as_bool()
            .unwrap()
    );
    held.call(&["auth", "begin"]).unwrap();
    assert!(
        held.call(&["auth", "status"]).unwrap().value["pending"]
            .as_bool()
            .unwrap()
    );
}

#[cfg(unix)]
#[test]
fn permissions() {
    use std::os::unix::fs::PermissionsExt;
    let held = Fixture::new();
    Private::current().create(&held.state, b"fixture").unwrap();
    let link = held.state.with_file_name("symlink.json");
    std::os::unix::fs::symlink(&held.state, &link).unwrap();
    assert!(Private::current().read(&link, 1024).is_err());
    std::fs::set_permissions(&held.state, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Private::current().read(&held.state, 1024).is_err());
}
