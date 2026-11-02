#![cfg(unix)]
use super::*;
use crate::{
    custody::Private,
    tencent::{
        Client,
        model::{Credential, State, stamp},
        secret::Store,
    },
};
use std::os::unix::fs::PermissionsExt;

struct Case {
    account: &'static str,
    stamped: bool,
    expires: Option<u64>,
    class: Class,
}

#[test]
fn binding() {
    let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("state.json");
    for Case {
        account,
        stamped,
        expires,
        class,
    } in [
        Case {
            account: "123",
            stamped: true,
            expires: None,
            class: Class::Input,
        },
        Case {
            account: "999",
            stamped: true,
            expires: None,
            class: Class::Binding,
        },
        Case {
            account: "123",
            stamped: false,
            expires: None,
            class: Class::Binding,
        },
        Case {
            account: "123",
            stamped: true,
            expires: Some(1),
            class: Class::Expired,
        },
    ] {
        let credential = Credential {
            id: "AKID_FIXTURE".into(),
            key: "PRIVATE_MARKER".into(),
            token: None,
            expires,
        };
        let mut state = State::fresh();
        state.identity = Some(
            json!({"AccountId":account,"PrincipalId":"456","Type":"CAMUser",
            "credentialStamp":if stamped { stamp(&credential) } else { "foreign".into() },
            "verifiedAt":1,"RequestId":"identity-id"}),
        );
        state.credential = Some(credential);
        Private::current()
            .replace(&path, &serde_json::to_vec(&state).unwrap())
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let client = Client::open(&path, "123").unwrap();
        assert!(Store::open(&path).is_err());
        for mut request in [
            super::instance(),
            super::key(),
            super::keys(),
            super::mutation::import(),
            super::mutation::associate(),
            super::mutation::delete(),
        ] {
            if class == Class::Input {
                request.region.clear()
            }
            let error = client.lighthouse(&request).unwrap_err();
            assert_eq!(error.outcome, Outcome::Unsent);
            assert_eq!(error.fault.class, class);
            assert!(!format!("{error:?} {error}").contains("PRIVATE_MARKER"));
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
        drop(client);
        assert!(Store::open(&path).is_ok());
    }
}
