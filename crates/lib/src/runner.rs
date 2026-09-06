use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::core::{config::Config, injections::Patch};

pub(super) struct Runner;

impl Runner {
    pub(super) fn run(config: &Config, argv: &[String], env: &Patch) -> Result<i32> {
        if argv.is_empty() {
            bail!("profile mode requires a command");
        }
        let mut child = Command::new(&argv[0]);
        child.args(&argv[1..]);
        for key in &env.unset {
            child.env_remove(key);
        }
        child.envs(&env.set);
        for key in [
            "RUNSEAL_HOME",
            "RUNSEAL_PROFILE",
            "RUNSEAL_PROFILE_PATH",
            "RUNSEAL_ROOT",
        ] {
            child.env_remove(key);
        }
        child.env("RUNSEAL_HOME", &config.home);
        child.env("RUNSEAL_PROFILE", config.label());
        child.env("RUNSEAL_ROOT", &config.root);
        if let Some(path) = &config.path {
            child.env("RUNSEAL_PROFILE_PATH", path);
        }

        let status = child
            .status()
            .context("failed to execute profiled command")?;
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
}
