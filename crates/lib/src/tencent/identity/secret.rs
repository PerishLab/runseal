use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

use super::model::State;
use crate::custody::Private;

pub(in crate::tencent) struct Store {
    pub path: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Tencent state requires an absolute private path"))?;
        Private::current().folder(parent).map_err(|_| {
            anyhow::anyhow!(
                "Tencent state custody refused; use a private directory under trusted ancestors"
            )
        })?;
        let lock = Private::current()
            .lock(&path.with_extension("lock"))
            .map_err(|_| anyhow::anyhow!("Tencent state is unsafe or already in use"))?;
        Ok(Self {
            path: path.to_owned(),
            _lock: lock,
        })
    }

    pub fn read(&self) -> Result<State> {
        let bytes = read(&self.path)?;
        let state: State =
            serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Tencent state refused"))?;
        if state.version != 1 {
            bail!("Tencent state version refused");
        }
        Ok(state)
    }

    pub fn save(&self, state: &State) -> Result<()> {
        let bytes = Zeroizing::new(
            serde_json::to_vec(state)
                .map_err(|_| anyhow::anyhow!("Tencent state encoding refused"))?,
        );
        Private::current().replace(&self.path, &bytes).map_err(|_| {
            anyhow::anyhow!(
                "Tencent state could not be committed; the remote operation may have completed"
            )
        })
    }

    pub fn create(&self, state: &State) -> Result<()> {
        let bytes = Zeroizing::new(
            serde_json::to_vec(state)
                .map_err(|_| anyhow::anyhow!("Tencent state encoding refused"))?,
        );
        Private::current()
            .create(&self.path, &bytes)
            .map_err(|_| anyhow::anyhow!("Tencent state destination must be new and private"))
    }
}

pub(in crate::tencent) fn read(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
    Private::current()
        .read(path, 256 * 1024)
        .map(Zeroizing::new)
        .map_err(|_| anyhow::anyhow!("Tencent private input custody refused"))
}

pub(in crate::tencent) struct Reserved {
    file: crate::custody::Reserved,
}

impl Reserved {
    pub fn open(path: &Path) -> Result<Self> {
        let file = Private::current().output(path).map_err(|_| {
            anyhow::anyhow!("Tencent output must be a new private file under trusted ancestors")
        })?;
        Ok(Self { file })
    }

    pub fn commit(self, state: &State, receipt: Value) -> Result<Value> {
        let bytes = Zeroizing::new(
            serde_json::to_vec(state)
                .map_err(|_| anyhow::anyhow!("Tencent secret encoding refused"))?,
        );
        if self.file.commit(&bytes).is_err() {
            bail!(
                "Tencent key was issued but private delivery failed; use the explicit target user and issuance receipt to inspect keys before any recovery; receipt: {receipt}"
            );
        }
        Ok(receipt)
    }
}
