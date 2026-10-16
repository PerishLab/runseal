use std::{env, process};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, error::ErrorKind};
use runseal::{
    Execution,
    core::{config::Config, route::Route},
    inspect, resolve,
};

#[cfg(feature = "managed-skill")]
mod skill;

mod observation;

#[derive(Parser)]
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

Optional Locus observation:
  Set [locus] enabled = true in <Runseal data home>/config.toml.
  endpoint defaults to LOCUS_API or http://127.0.0.1:43308.
  Buffer: <Runseal data home>/state/locus. Missing configuration disables it.
  Profile files remain env, argv, and symlink only.
  External preparation emits one bounded, source-less prepared.v1 fact.
  It classifies gh/concord/plumb, simple env assignments and public verbs;
  only GODEBUG=http2client=0 is retained as an explicit assignment value.
  Other input is classified without retaining arguments, paths or environment.
  This records preparation, not child startup or the effective environment.

Repository: https://github.com/PerishLab/runseal"
)]
struct Cli {
    #[command(subcommand)]
    command: Control,
}

#[derive(Subcommand)]
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
    #[cfg(feature = "managed-skill")]
    #[command(about = "Manage Runseal agent skill installations")]
    Skill {
        #[command(subcommand)]
        deed: skill::Deed,
    },
}

fn main() -> Result<()> {
    #[cfg(feature = "managed-skill")]
    plumb::identity!("RUNSEAL").map_err(anyhow::Error::msg)?;
    let cwd = env::current_dir().context("failed to read current directory")?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = match args.first().map(String::as_str) {
        Some(value) if value.starts_with(':') => {
            if args.get(1).is_some_and(|value| value.starts_with('@')) {
                "tool"
            } else {
                "profile.run"
            }
        }
        Some("profile") => "profile",
        Some("resolve") => "resolve",
        Some("skill") => "skill",
        _ => "control",
    };
    let observation = observation::start(&cwd, command);
    let result = execute(args, &cwd);
    if let Some(observation) = observation {
        observation.finish(match &result {
            Ok(code) => code.unwrap_or(0),
            Err(_) => 1,
        });
    }
    if let Some(code) = result? {
        process::exit(code);
    }
    Ok(())
}

#[locus::trace(with = plumb::observation::view())]
fn execute(args: Vec<String>, cwd: &std::path::Path) -> Result<Option<i32>> {
    match Route::parse(args)? {
        Route::Control(args) => control(args, cwd),
        Route::Profile { name, command } => profile(name, command, cwd),
    }
}

#[locus::trace(with = plumb::observation::view(), frame = frame)]
fn profile(
    name: Option<String>,
    command: Vec<String>,
    cwd: &std::path::Path,
) -> Result<Option<i32>> {
    ready()?;
    let config = Config::build(name, command, cwd)?;
    let mut execution = Execution::prepare(&config)?;
    if let Some(argv) = execution.argv() {
        observation::prepared(argv);
    }
    execution.handoff(observation::parent(frame.as_ref()));
    Ok(execution.run()?.code)
}

#[locus::trace(with = plumb::observation::view())]
fn control(args: Vec<String>, cwd: &std::path::Path) -> Result<Option<i32>> {
    let cli = match Cli::try_parse_from(std::iter::once("runseal".to_string()).chain(args)) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    ready()?;
    match cli.command {
        Control::Profile { name } => {
            inspect(&Config::build(name, Vec::new(), cwd)?)?;
            Ok(None)
        }
        Control::Resolve { profile, uri } => {
            resolve(&Config::build(profile, Vec::new(), cwd)?, &uri)?;
            Ok(None)
        }
        #[cfg(feature = "managed-skill")]
        Control::Skill { deed } => {
            let code = skill::run(deed);
            Ok((code != 0).then_some(code))
        }
    }
}

#[cfg(feature = "managed-skill")]
fn version() -> &'static str {
    plumb::version!("RUNSEAL")
}

#[cfg(not(feature = "managed-skill"))]
fn version() -> &'static str {
    option_env!("RUNSEAL_BUILD_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
}

fn ready() -> Result<()> {
    #[cfg(feature = "managed-skill")]
    plumb::identity::ready().map_err(anyhow::Error::msg)?;
    Ok(())
}
