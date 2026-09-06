use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::core::profile::Symlink as Spec;

pub(crate) struct Symlink {
    cfg: Spec,
    registered: bool,
}

impl Symlink {
    pub(crate) fn new(cfg: Spec) -> Self {
        Self {
            cfg,
            registered: false,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !self.cfg.source.exists() {
            bail!("source does not exist: {}", self.cfg.source.display());
        }
        match std::fs::symlink_metadata(&self.cfg.target) {
            Ok(_) => bail!(
                "refusing occupied symlink target: {}",
                self.cfg.target.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn register(&mut self) -> Result<()> {
        if let Some(parent) = self.cfg.target.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create symlink parent: {}", parent.display())
            })?;
        }
        link(&self.cfg.source, &self.cfg.target)
            .with_context(|| format!("failed to create symlink: {}", self.cfg.target.display()))?;
        self.registered = true;
        Ok(())
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        if !self.registered {
            return Ok(());
        }
        let metadata = std::fs::symlink_metadata(&self.cfg.target)
            .with_context(|| format!("failed to inspect symlink: {}", self.cfg.target.display()))?;
        if !metadata.file_type().is_symlink() {
            bail!(
                "refusing to remove non-symlink at {}",
                self.cfg.target.display()
            );
        }
        let held = std::fs::read_link(&self.cfg.target)
            .with_context(|| format!("failed to read symlink: {}", self.cfg.target.display()))?;
        if held != self.cfg.source {
            bail!(
                "refusing to remove symlink with unexpected source: {}",
                self.cfg.target.display()
            );
        }
        std::fs::remove_file(&self.cfg.target)
            .with_context(|| format!("failed to remove symlink: {}", self.cfg.target.display()))?;
        self.registered = false;
        Ok(())
    }
}

#[cfg(unix)]
fn link(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn link(source: &Path, target: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, target)
    } else {
        std::os::windows::fs::symlink_file(source, target)
    }
}
