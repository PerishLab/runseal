use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::core::profile::{Existing, Symlink as Spec};

pub(crate) struct Symlink {
    cfg: Spec,
    cleanup: bool,
}

impl Symlink {
    pub(crate) fn new(cfg: Spec) -> Self {
        Self {
            cfg,
            cleanup: false,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        "symlink"
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.cfg.source.to_string_lossy().trim().is_empty() {
            bail!("source must not be empty");
        }
        if self.cfg.target.to_string_lossy().trim().is_empty() {
            bail!("target must not be empty");
        }
        if !self.cfg.source.exists() {
            bail!("source does not exist: {}", self.cfg.source.display());
        }
        Ok(())
    }

    pub(crate) fn register(&mut self) -> Result<()> {
        self.create(&self.cfg.source, &self.cfg.target, self.cfg.existing)?;
        self.cleanup = true;
        Ok(())
    }

    pub(crate) fn export(&self) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        if self.cleanup && self.cfg.cleanup {
            self.remove(&self.cfg.target, &self.cfg.source)?;
        }
        self.cleanup = false;
        Ok(())
    }

    fn create(&self, source: &Path, target: &Path, existing: Existing) -> Result<()> {
        match std::fs::symlink_metadata(target) {
            Ok(meta) => conflict(target, existing, meta)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create symlink parent: {}", parent.display())
            })?;
        }
        link(source, target).with_context(|| context("create", target))?;
        Ok(())
    }

    fn remove(&self, target: &Path, source: &Path) -> Result<()> {
        let metadata = std::fs::symlink_metadata(target)
            .with_context(|| context("inspect during shutdown", target))?;
        if !metadata.file_type().is_symlink() {
            bail!("refusing to remove non-symlink at {}", target.display());
        }
        let link =
            std::fs::read_link(target).with_context(|| context("read during shutdown", target))?;
        if link != source {
            bail!(
                "refusing to remove symlink with unexpected target: {}",
                target.display()
            );
        }
        std::fs::remove_file(target).with_context(|| context("remove during shutdown", target))?;
        Ok(())
    }
}

fn conflict(target: &Path, existing: Existing, meta: std::fs::Metadata) -> Result<()> {
    match existing {
        Existing::Error => bail!("refusing to overwrite existing file: {}", target.display()),
        Existing::Replace => replace(target, meta),
    }
}

fn replace(target: &Path, meta: std::fs::Metadata) -> Result<()> {
    if meta.file_type().is_dir() {
        bail!("refusing to replace directory target: {}", target.display());
    }
    std::fs::remove_file(target).with_context(|| context("replace", target))
}

fn context(action: &str, target: &Path) -> String {
    format!(
        "failed to {action} symlink target {}; lifecycle symlink targets are single-owner and may already be managed by another concurrent runseal process",
        target.display()
    )
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
