use anyhow::{Context, Result, bail};
use std::process::Command;

use super::super::config::Config;
use super::super::profile::{Deno, Profile};
use super::Resolved;
use super::wrappers;

pub(super) struct Runner;

impl Runner {
    pub(super) fn run(
        config: &Config,
        profile: &Profile,
        resolved: &Resolved,
        exports: &[(String, String)],
    ) -> Result<i32> {
        let command = &resolved.argv;
        if command.is_empty() {
            bail!("command mode requires at least one command token");
        }
        if let Some(wrapper) = &resolved.wrapper
            && (wrappers::Bin {
                path: &wrapper.file,
            })
            .deno()
        {
            return Self::deno(config, profile.deno.as_ref(), resolved, exports);
        }

        let mut child = Self::child(resolved);
        child.env_remove("RUNSEAL_WRAPPER_NAME");
        child.env_remove("RUNSEAL_WRAPPER_FILE");
        child.envs(Self::env(config, resolved, exports)?);
        Self::wait(child)
    }

    fn deno(
        config: &Config,
        deno: Option<&Deno>,
        resolved: &Resolved,
        exports: &[(String, String)],
    ) -> Result<i32> {
        let deno =
            deno.ok_or_else(|| anyhow::anyhow!("deno wrapper requires a [deno] profile policy"))?;
        let wrapper = resolved
            .wrapper
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("deno wrapper execution requires a resolved wrapper"))?;
        let mut child = Command::new("deno");
        child.arg("run").arg("--no-prompt");
        if let Some(config) = &deno.config {
            child.arg("--config").arg(config);
        }
        if let Some(lock) = &deno.lock {
            child.arg("--lock").arg(lock);
            child.arg("--frozen=true");
        }
        child.args(super::permissions::Permissions::expand(&deno.permissions)?);
        child.arg(&wrapper.file);
        if resolved.argv.len() > 1 {
            child.args(&resolved.argv[1..]);
        }
        child.env_remove("RUNSEAL_WRAPPER_NAME");
        child.env_remove("RUNSEAL_WRAPPER_FILE");
        child.envs(Self::env(config, resolved, exports)?);
        Self::wait(child).context("failed to execute deno wrapper")
    }

    fn wait(mut child: Command) -> Result<i32> {
        let status = child.status().context("failed to execute child command")?;
        if let Some(code) = status.code() {
            return Ok(code);
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Ok(128 + signal);
            }
        }

        Ok(1)
    }

    fn env(
        config: &Config,
        resolved: &Resolved,
        exports: &[(String, String)],
    ) -> Result<Vec<(String, String)>> {
        let mut env = exports.to_vec();
        env.push((
            "RUNSEAL_HOME".to_string(),
            config.home.to_string_lossy().into_owned(),
        ));
        env.push((
            "RUNSEAL_PROFILE_HOME".to_string(),
            config.profiles.to_string_lossy().into_owned(),
        ));
        env.push((
            "RUNSEAL_PROFILE_PATH".to_string(),
            config.profile.to_string_lossy().into_owned(),
        ));
        env.push((
            "RUNSEAL_WRAPPER_PATH".to_string(),
            (wrappers::Fleet { config })
                .env()?
                .to_string_lossy()
                .into_owned(),
        ));
        if let Some(wrapper) = &resolved.wrapper {
            env.push(("RUNSEAL_WRAPPER_NAME".to_string(), wrapper.name.clone()));
            env.push((
                "RUNSEAL_WRAPPER_FILE".to_string(),
                wrapper.file.to_string_lossy().into_owned(),
            ));
        }
        Ok(env)
    }

    fn child(resolved: &Resolved) -> Command {
        #[cfg(windows)]
        if let Some(wrapper) = &resolved.wrapper
            && Self::cmd(&wrapper.file)
        {
            let mut child = Command::new("cmd");
            child.arg("/C").arg(&wrapper.file);
            if resolved.argv.len() > 1 {
                child.args(&resolved.argv[1..]);
            }
            return child;
        }

        let mut child = Command::new(&resolved.argv[0]);
        if resolved.argv.len() > 1 {
            child.args(&resolved.argv[1..]);
        }
        child
    }

    #[cfg(windows)]
    fn cmd(path: &std::path::Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some(ext) if ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat")
        )
    }
}
