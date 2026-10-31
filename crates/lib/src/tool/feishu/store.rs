use super::{
    Class, Error,
    model::{Import, State, identifier, visible},
};
use crate::custody::Private;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub(super) struct Store {
    path: PathBuf,
    _lock: radicula_lock::file::Permit,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(custody());
        }
        let parent = path.parent().ok_or_else(custody)?;
        Private::current().folder(parent).map_err(|_| custody())?;
        let lock = Private::current()
            .hold(&path.with_extension("lock"))
            .map_err(|_| custody())?;
        Ok(Self {
            path: path.into(),
            _lock: lock,
        })
    }

    pub fn read(&self, app: &str) -> Result<State, Error> {
        identifier(app, "cli_")?;
        let bytes = Zeroizing::new(
            Private::current()
                .read(&self.path, 256 * 1024)
                .map_err(|_| custody())?,
        );
        let state: State = serde_json::from_slice(&bytes).map_err(|_| custody())?;
        if !matches!(state.version, 1..=3) || state.app != app {
            return Err(Error::new(
                Class::Binding,
                "Feishu private state does not match the expected app/version",
            ));
        }
        if state.version == 1 && state.user.is_some() {
            return Err(custody());
        }
        if state.version < 3
            && state
                .user
                .as_ref()
                .is_some_and(|user| user.returned.is_some())
        {
            return Err(custody());
        }
        visible(&state.secret, 256)?;
        Ok(state)
    }

    pub fn import(&self, app: &str, path: &Path) -> Result<State, Error> {
        identifier(app, "cli_")?;
        let bytes = Zeroizing::new(
            Private::current()
                .read(path, 16 * 1024)
                .map_err(|_| custody())?,
        );
        let held: Import = serde_json::from_slice(&bytes).map_err(|_| custody())?;
        if held.app != app {
            return Err(Error::new(
                Class::Binding,
                "Feishu credential import does not match the expected app",
            ));
        }
        visible(&held.secret, 256)?;
        let state = State {
            version: 1,
            app: app.into(),
            secret: held.secret.clone(),
            credential: None,
            identity: None,
            user: None,
        };
        let bytes = encode(&state)?;
        Private::current()
            .create(&self.path, &bytes)
            .map_err(|_| custody())?;
        Ok(state)
    }

    pub fn save(&self, state: &State) -> Result<(), Error> {
        let bytes = encode(state)?;
        Private::current()
            .replace(&self.path, &bytes)
            .map_err(|_| Error::unknown(Class::Custody))
    }
}

fn encode(state: &State) -> Result<Zeroizing<Vec<u8>>, Error> {
    serde_json::to_vec(state)
        .map(Zeroizing::new)
        .map_err(|_| custody())
}

fn custody() -> Error {
    Error::new(
        Class::Custody,
        "Feishu custody refused; use an absolute private state/input under trusted ancestors, a new import destination and no competing operation",
    )
}
