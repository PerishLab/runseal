use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::core::profile::{SymlinkOnExist, SymlinkProfile};

pub(crate) struct SymlinkInjection {
    cfg: SymlinkProfile,
    cleanup_target: bool,
}

impl SymlinkInjection {
    pub(crate) fn new(cfg: SymlinkProfile) -> Self {
        Self {
            cfg,
            cleanup_target: false,
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
        self.register_at(&self.cfg.source, &self.cfg.target, self.cfg.on_exist)?;
        self.cleanup_target = true;
        Ok(())
    }

    pub(crate) fn export(&self) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        if self.cleanup_target && self.cfg.cleanup {
            self.shutdown_at(&self.cfg.target, &self.cfg.source)?;
        }
        self.cleanup_target = false;
        Ok(())
    }

    fn register_at(&self, source: &Path, target: &Path, on_exist: SymlinkOnExist) -> Result<()> {
        match std::fs::symlink_metadata(target) {
            Ok(meta) => existing(target, on_exist, meta)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create symlink parent: {}", parent.display())
            })?;
        }
        create_symlink(source, target).with_context(|| shared_target_context("create", target))?;
        Ok(())
    }

    fn shutdown_at(&self, target: &Path, source: &Path) -> Result<()> {
        let metadata = std::fs::symlink_metadata(target)
            .with_context(|| shared_target_context("inspect during shutdown", target))?;
        if !metadata.file_type().is_symlink() {
            bail!("refusing to remove non-symlink at {}", target.display());
        }
        let link_target = std::fs::read_link(target)
            .with_context(|| shared_target_context("read during shutdown", target))?;
        if link_target != source {
            bail!(
                "refusing to remove symlink with unexpected target: {}",
                target.display()
            );
        }
        std::fs::remove_file(target)
            .with_context(|| shared_target_context("remove during shutdown", target))?;
        Ok(())
    }
}

fn existing(target: &Path, on_exist: SymlinkOnExist, meta: std::fs::Metadata) -> Result<()> {
    match on_exist {
        SymlinkOnExist::Error => bail!("refusing to overwrite existing file: {}", target.display()),
        SymlinkOnExist::Replace => replace(target, meta),
    }
}

fn replace(target: &Path, meta: std::fs::Metadata) -> Result<()> {
    if meta.file_type().is_dir() {
        bail!("refusing to replace directory target: {}", target.display());
    }
    std::fs::remove_file(target).with_context(|| shared_target_context("replace", target))
}

fn shared_target_context(action: &str, target: &Path) -> String {
    format!(
        "failed to {action} symlink target {}; lifecycle symlink targets are single-owner and may already be managed by another concurrent runseal process",
        target.display()
    )
}

#[cfg(unix)]
fn create_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn create_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, target)
    } else {
        std::os::windows::fs::symlink_file(source, target)
    }
}
