use anyhow::{Context, Result, bail};

use crate::core::{
    config::Config,
    injections,
    profile::{self, Profile},
    symbol,
};

#[path = "runner.rs"]
mod runner;

pub struct Outcome {
    pub code: Option<i32>,
}

pub fn run(config: &Config) -> Result<Outcome> {
    let profile = Profile::load(config)?;
    let command = Command::resolve(&config.command, &profile)?;
    injections::Lifecycle::with(&profile, |env| match &command {
        Command::External(argv) => {
            let code = runner::Runner::run(config, argv, env)?;
            Ok(Outcome { code: Some(code) })
        }
    })
}

pub fn inspect(config: &Config) -> Result<()> {
    let profile = Profile::load(config)?;
    println!("RUNSEAL_HOME={}", config.home.display());
    println!("RUNSEAL_PROFILE={}", config.label());
    println!("RUNSEAL_ROOT={}", config.root.display());
    if let Some(path) = &config.path {
        println!("RUNSEAL_PROFILE_PATH={}", path.display());
    }
    println!("RUNSEAL_ENV={}", profile.env.vars.len());
    println!("RUNSEAL_ARGV={}", profile.argv.len());
    println!("RUNSEAL_SYMLINK={}", profile.symlink.len());
    Ok(())
}

pub fn resolve(config: &Config, uris: &[String]) -> Result<()> {
    for uri in uris {
        println!("{}", profile::resolve(&config.root, uri)?.display());
    }
    Ok(())
}

enum Command {
    External(Vec<String>),
}

impl Command {
    fn resolve(command: &[String], profile: &Profile) -> Result<Self> {
        if command.is_empty() {
            bail!("profile mode requires a command or @tool");
        }
        let argv = Args::apply(command, profile)?;
        let Some(name) = argv[0].strip_prefix('@') else {
            return Ok(Self::External(argv));
        };
        if name.is_empty() {
            bail!("@tool name must not be empty");
        }
        symbol::valid(name).with_context(|| format!("invalid @tool name: @{name}"))?;
        bail!("unknown Runseal tool: @{name}")
    }
}

struct Args;

impl Args {
    fn apply(command: &[String], profile: &Profile) -> Result<Vec<String>> {
        let Some(injected) = profile.argv.get(&command[0]) else {
            return Ok(command.to_vec());
        };
        if injected.is_empty() {
            bail!("argv injection for {} must not be empty", command[0]);
        }
        let mut rewritten = Vec::with_capacity(command.len() + injected.len());
        rewritten.push(command[0].clone());
        rewritten.extend(injected.iter().cloned());
        rewritten.extend(command[1..].iter().cloned());
        Ok(rewritten)
    }
}
