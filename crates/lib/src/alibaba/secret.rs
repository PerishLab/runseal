use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

use super::model::{STATE, State};
use crate::custody::Private;

pub(super) struct Store {
    pub path: PathBuf,
    _permit: radicula_lock::file::Permit,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            bail!("Alibaba state requires an absolute private path");
        }
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Alibaba state requires an absolute private path"))?;
        Private::current().folder(parent).map_err(|_| {
            anyhow::anyhow!(
                "Alibaba state custody refused; use a private directory under trusted ancestors"
            )
        })?;
        let permit = Private::current()
            .hold(&path.with_extension("lock"))
            .map_err(|_| anyhow::anyhow!("Alibaba state is unsafe or already in use"))?;
        Ok(Self {
            path: path.to_owned(),
            _permit: permit,
        })
    }

    pub fn exists(&self) -> Result<bool> {
        self.path
            .try_exists()
            .map_err(|_| anyhow::anyhow!("Alibaba state presence could not be checked"))
    }

    pub fn read(&self) -> Result<State> {
        let bytes = read(&self.path)?;
        let state: State =
            serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Alibaba state refused"))?;
        if !matches!(state.version, 1 | 2) || state.site != super::model::SITE {
            bail!("Alibaba state version or site refused");
        }
        state.check()?;
        Ok(state)
    }

    pub fn save(&self, state: &State) -> Result<()> {
        let bytes = encode(state)?;
        Private::current().replace(&self.path, &bytes).map_err(|_| {
            anyhow::anyhow!(
                "Alibaba state could not be committed; the remote token operation may have completed; inspect status before any recovery"
            )
        })
    }

    pub fn create(&self, state: &State) -> Result<()> {
        let bytes = encode(state)?;
        Private::current()
            .create(&self.path, &bytes)
            .map_err(|_| anyhow::anyhow!("Alibaba state destination must be new and private"))
    }
}

fn encode(state: &State) -> Result<Zeroizing<Vec<u8>>> {
    state.check()?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(state).map_err(|_| anyhow::anyhow!("Alibaba state encoding refused"))?,
    );
    if bytes.len() > STATE {
        bail!("Alibaba state exceeds the private state allowance");
    }
    Ok(bytes)
}

pub(super) fn read(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
    if !path.is_absolute() {
        bail!("Alibaba private input must be an absolute path");
    }
    Private::current()
        .read(path, STATE)
        .map(Zeroizing::new)
        .map_err(|_| anyhow::anyhow!("Alibaba private input custody refused"))
}
