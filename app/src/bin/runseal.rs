use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use runseal::core::app::App;
use runseal::core::config::{Config, Env, Input};
use runseal::core::help;
use runseal::run;

#[derive(Debug, Parser)]
#[command(
    name = "runseal",
    version = version(),
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
  @wrappers           list visible wrappers
  @which :<name>      print a wrapper path

Deno wrappers:
  .ts files are run with deno using the repo-level [deno] profile policy.
  Use TypeScript for structured operations over the harness library.

Profile discovery walks from the current directory upward for runseal.toml|yaml|yml|json,
then falls back to $RUNSEAL_PROFILE_HOME/default.toml|yaml|yml|json.

Run runseal @profile --help, @resolve --help, @wrappers --help,
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
    cli.command = normalize(cli.command);
    if cli.command.is_empty() {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }
    if help(&cli.command)? {
        return Ok(());
    }

    let config = config(cli)?;
    let app = App::new(config);
    let result = run(&app)?;
    if let Some(code) = result.code {
        process::exit(code);
    }
    Ok(())
}

fn config(cli: Cli) -> Result<Config> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    Config::build(
        Input {
            profile: cli.profile,
            command: normalize(cli.command),
        },
        Env::process(),
        &cwd,
    )
}

fn help(command: &[String]) -> Result<bool> {
    let Some(name) = command[0].strip_prefix('@') else {
        return Ok(false);
    };
    if name.is_empty() {
        bail!("internal command name must not be empty");
    }
    let Some(help) = help::resolve(name, &command[1..])? else {
        return Ok(false);
    };
    print!("{help}");
    Ok(true)
}

fn normalize(mut command: Vec<String>) -> Vec<String> {
    if command.len() > 1 && command.get(1).map(String::as_str) == Some("--") {
        command.remove(1);
    }
    command
}

fn version() -> &'static str {
    option_env!("RUNSEAL_BUILD_VERSION").unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
}
