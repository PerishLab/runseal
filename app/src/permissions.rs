use anyhow::{Context, Result};

pub(super) struct Permissions;

impl Permissions {
    pub(super) fn expand(permissions: &[String]) -> Result<Vec<String>> {
        permissions
            .iter()
            .map(|permission| {
                shellexpand::env(permission)
                    .map(|expanded| expanded.into_owned())
                    .with_context(|| format!("unable to expand deno permission: {permission}"))
            })
            .collect()
    }
}
