mod identity;
mod protocol;
mod recovery;
mod state;

use super::model::{self, Credential, Identity, stamp};
use serde_json::{Value, json};
use std::path::PathBuf;

struct Fixture {
    _root: tempfile::TempDir,
    state: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        #[cfg(unix)]
        let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
        #[cfg(windows)]
        let root = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let folder = std::fs::canonicalize(root.path()).unwrap().join("private");
        crate::custody::Private::current().folder(&folder).unwrap();
        Self {
            state: folder.join("state.json"),
            _root: root,
        }
    }

    fn store(&self) -> super::secret::Store {
        super::secret::Store::open(&self.state).unwrap()
    }
}

fn credential() -> Credential {
    Credential {
        id: "STS.fixture-id".into(),
        key: "PRIVATE_KEY_MARKER".into(),
        token: "PRIVATE_TOKEN_MARKER".into(),
        expires: model::now() + 900,
        observed: 1,
    }
}

fn caller() -> Value {
    json!({
        "RequestId": "fixture-request-id",
        "IdentityType": "RAMUser",
        "AccountId": "1234567890",
        "PrincipalId": "1234567890123456",
        "UserId": "2345678901234567",
        "Arn": "acs:ram::1234567890:user/fixture-user",
    })
}

fn identity() -> Identity {
    Identity {
        account: "1234567890".into(),
        principal: "1234567890123456".into(),
        user: "2345678901234567".into(),
        kind: "RAMUser".into(),
        arn: "acs:ram::1234567890:user/fixture-user".into(),
        stamp: stamp(&credential()),
        observed: 1,
        request: "fixture-request-id".into(),
    }
}

fn base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
