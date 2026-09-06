use std::{env, process};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, error::ErrorKind};
use runseal::{
    core::{config::Config, route::Route},
    inspect, resolve, run,
};

mod cookbook;
#[cfg(feature = "managed-skill")]
mod skill;

#[derive(Debug, Parser)]
#[command(
    name = "runseal",
    version = version(),
    about = "Establish one profile and run one isolated command.",
    arg_required_else_help = true,
    after_help = "\
Execution model:
  runseal <internal-command>       operate Runseal's control plane
  runseal : <command> [args...]   run inside the default profile
  runseal :name <command> [...]   run inside a named profile
  runseal :name @tool [...]       run a Runseal-owned atomic tool

A profile contains only env, argv, and symlink declarations. It carries no
command, wrapper, task graph, or implicit orchestration.

Repository: https://git.perish.top/PerishFire/runseal"
)]
struct Cli {
    #[command(subcommand)]
    command: Control,
}

#[derive(Debug, Subcommand)]
enum Control {
    #[command(about = "Inspect a resolved profile without applying it")]
    Profile {
        #[arg(help = "Named profile; omit for default")]
        name: Option<String>,
    },
    #[command(about = "Resolve conventional resource:// or local:// paths")]
    Resolve {
        #[arg(short, long, help = "Named profile; omit for default")]
        profile: Option<String>,
        #[arg(required = true, help = "One or more profile paths")]
        uri: Vec<String>,
    },
    #[command(about = "Explain recovery for a Runseal refusal")]
    Cookbook {
        #[arg(help = "Recovery entry; omit to list the available entries")]
        entry: Option<String>,
    },
    #[cfg(feature = "managed-skill")]
    #[command(about = "Manage Runseal agent skill installations")]
    Skill {
        #[command(subcommand)]
        deed: skill::Deed,
    },
}

fn main() -> Result<()> {
    let cwd = env::current_dir().context("failed to read current directory")?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    match Route::parse(args)? {
        Route::Control(args) => control(args, &cwd),
        Route::Profile { name, command } => {
            let config = Config::build(name, command, &cwd)?;
            let outcome = run(&config)?;
            if let Some(code) = outcome.code {
                process::exit(code);
            }
            Ok(())
        }
    }
}

fn control(args: Vec<String>, cwd: &std::path::Path) -> Result<()> {
    let cli = match Cli::try_parse_from(std::iter::once("runseal".to_string()).chain(args)) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    match cli.command {
        Control::Profile { name } => inspect(&Config::build(name, Vec::new(), cwd)?),
        Control::Resolve { profile, uri } => {
            resolve(&Config::build(profile, Vec::new(), cwd)?, &uri)
        }
        Control::Cookbook { entry } => {
            print!("{}", cookbook::render(entry.as_deref())?);
            Ok(())
        }
        #[cfg(feature = "managed-skill")]
        Control::Skill { deed } => {
            let code = skill::run(deed);
            if code != 0 {
                process::exit(code);
            }
            Ok(())
        }
    }
}

fn version() -> &'static str {
    option_env!("RUNSEAL_BUILD_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
}
