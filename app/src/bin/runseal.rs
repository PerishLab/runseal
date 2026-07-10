use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use runseal::core::app::App;
use runseal::core::config::{Config, Env, Input};
use runseal::core::internal_help;
use runseal::core::tool;
use runseal::run;

#[derive(Debug, Parser)]
#[command(
    name = "runseal",
    version = build_version(),
    about = "Run a command inside an env, symlink, argv, and wrapper profile.",
    after_help = "\
Command model:
  runseal <cmd>       run an external command inside the profile
  runseal :<name>     run a profile wrapper
  runseal @<name>     run a runseal command

Runseal commands:
  @profile            print resolved runtime paths
  @resources          print the resolved resource root
  @resolve <uri>...   resolve resource:// paths
  @tool               run an atomic runseal tool command
  @wrappers           list visible wrappers
  @which :<name>      print a wrapper path

Deno wrappers:
  .ts files are run with deno using the repo-level [deno] profile policy.
  Use TypeScript for structured operations and runseal @tool for atomic glue.

Profile discovery walks from the current directory upward for runseal.toml|yaml|yml|json,
then falls back to $RUNSEAL_PROFILE_HOME/default.toml|yaml|yml|json.

Run runseal @profile --help, @resolve --help, @tool --help, @wrappers --help,
or @which --help for details.

Repository: https://git.perish.top/PerishFire/runseal"
)]
struct Cli {
    #[arg(short = 'p', long = "profile")]
    profile: Option<PathBuf>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    cli.command = normalize_command(cli.command);
    if cli.command.is_empty() {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }
    if print_internal_help(&cli.command)? {
        return Ok(());
    }
    if run_early_internal(&cli.command)? {
        return Ok(());
    }

    let config = build_runtime_config(cli)?;
    let app = App::new(config);
    let result = run(&app)?;
    if let Some(code) = result.exit_code {
        process::exit(code);
    }
    Ok(())
}

fn build_runtime_config(cli: Cli) -> Result<Config> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    Config::build(
        Input {
            profile: cli.profile,
            command: normalize_command(cli.command),
        },
        Env::process(),
        &cwd,
    )
}

fn run_early_internal(command: &[String]) -> Result<bool> {
    match command.first().map(String::as_str) {
        Some("@tool") => {
            tool::run(&command[1..])?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn print_internal_help(command: &[String]) -> Result<bool> {
    let Some(name) = command[0].strip_prefix('@') else {
        return Ok(false);
    };
    if name.is_empty() {
        bail!("internal command name must not be empty");
    }
    let Some(help) = internal_help::resolve(name, &command[1..])? else {
        return Ok(false);
    };
    print!("{help}");
    Ok(true)
}

fn normalize_command(mut command: Vec<String>) -> Vec<String> {
    if command.len() > 1 && command.get(1).map(String::as_str) == Some("--") {
        command.remove(1);
    }
    command
}

fn build_version() -> &'static str {
    option_env!("RUNSEAL_BUILD_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
}
