use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, Result, bail};

use super::app::Context as App;
use super::config::Config;
use super::help;
use super::key;
use super::profile::{Injection, Profile};
use super::symbol;
use super::{injections, profile};

mod permissions;
mod runner;
mod wrappers;

pub struct Outcome {
    pub code: Option<i32>,
}

enum Internal {
    Help(&'static str),
    Profile,
    Resolve(Vec<String>),
    Resources,
    Wrappers,
    Which(String),
}

pub(crate) struct Resolved {
    argv: Vec<String>,
    wrapper: Option<Wrapper>,
}

struct Wrapper {
    name: String,
    file: PathBuf,
}

pub fn run(app: &dyn App) -> Result<Outcome> {
    let config = app.config();
    if let Some(command) = Parser::dispatch(config)? {
        command.run(config)?;
        return Ok(Outcome { code: Some(0) });
    }

    let profile = profile::load(&config.profile).context("unable to load runseal profile")?;
    let command = Parser::command(config, &profile)?;
    let outcome = injections::Lifecycle::with(app, profile.injections.clone(), |exports| {
        let env = Exports::map(exports.to_vec())?;
        let exports: Vec<(String, String)> = env.into_iter().collect();
        let code = runner::Runner::run(config, &profile, &command, &exports)?;
        Ok(Outcome { code: Some(code) })
    })?;
    Ok(outcome)
}

struct Parser;

impl Parser {
    fn dispatch(config: &Config) -> Result<Option<Internal>> {
        if config.command.is_empty() {
            bail!("command mode requires at least one command token");
        }
        let Some(name) = Self::internal(&config.command[0])? else {
            return Ok(None);
        };
        Ok(Some(Self::resolve(&name, &config.command[1..])?))
    }

    fn command(config: &Config, profile: &Profile) -> Result<Resolved> {
        if config.command.is_empty() {
            bail!("command mode requires at least one command token");
        }
        if let Some(name) = Self::wrapper(&config.command[0])? {
            let file = (wrappers::Fleet { config }).resolve(&name)?;
            let mut argv = Vec::with_capacity(config.command.len());
            argv.push(file.to_string_lossy().into_owned());
            argv.extend_from_slice(&config.command[1..]);
            return Ok(Resolved {
                argv,
                wrapper: Some(Wrapper { name, file }),
            });
        }

        Ok(Resolved {
            argv: Args::apply(&config.command, &profile.injections)?,
            wrapper: None,
        })
    }

    fn internal(token: &str) -> Result<Option<String>> {
        let Some(name) = token.strip_prefix('@') else {
            return Ok(None);
        };
        if name.is_empty() {
            bail!("internal command name must not be empty");
        }
        symbol::valid(name).with_context(|| format!("invalid internal command name: @{name}"))?;
        Ok(Some(name.to_string()))
    }

    fn resolve(name: &str, args: &[String]) -> Result<Internal> {
        if let Some(help) = help::resolve(name, args)? {
            return Ok(Internal::Help(help));
        }

        match name {
            "profile" => Self::validate(args, "@profile").map(|()| Internal::Profile),
            "resolve" => Self::uris(args),
            "resources" => Self::validate(args, "@resources").map(|()| Internal::Resources),
            "wrappers" => Self::validate(args, "@wrappers").map(|()| Internal::Wrappers),
            "which" => Self::which(args),
            _ => bail!("unknown internal command: @{name}"),
        }
    }

    fn uris(args: &[String]) -> Result<Internal> {
        if args.is_empty() {
            bail!("@resolve requires at least one resource:// URI argument");
        }
        Ok(Internal::Resolve(args.to_vec()))
    }

    fn which(args: &[String]) -> Result<Internal> {
        if args.len() != 1 {
            bail!("@which requires exactly one :wrapper argument");
        }
        let Some(name) = Self::wrapper(&args[0])? else {
            bail!("@which currently supports only :wrapper arguments");
        };
        Ok(Internal::Which(name))
    }

    fn validate(args: &[String], name: &str) -> Result<()> {
        if !args.is_empty() {
            bail!("{name} does not accept arguments");
        }
        Ok(())
    }

    fn wrapper(token: &str) -> Result<Option<String>> {
        let Some(name) = token.strip_prefix(':') else {
            return Ok(None);
        };
        if name.is_empty() {
            bail!("wrapper name must not be empty");
        }
        symbol::valid(name).with_context(|| format!("invalid wrapper name: :{name}"))?;
        Ok(Some(name.to_string()))
    }
}

impl Internal {
    fn run(self, config: &Config) -> Result<()> {
        match self {
            Self::Help(help) => print!("{help}"),
            Self::Profile => Self::profile(config)?,
            Self::Resolve(uris) => Self::resolve(config, &uris)?,
            Self::Resources => Self::resources(config)?,
            Self::Wrappers => Self::wrappers(config)?,
            Self::Which(name) => Self::which(config, &name)?,
        }
        Ok(())
    }

    fn profile(config: &Config) -> Result<()> {
        println!("RUNSEAL_HOME={}", config.home.display());
        println!("RUNSEAL_PROFILE_HOME={}", config.profiles.display());
        println!("RUNSEAL_PROFILE_PATH={}", config.profile.display());
        if let Ok(Some(resources)) = profile::resources(&config.profile)
            && let Ok(root) = profile::root(&config.profile, Some(&resources))
        {
            println!("RUNSEAL_RESOURCE_ROOT={}", root.display());
        }
        println!(
            "RUNSEAL_WRAPPER_PATH={}",
            (wrappers::Fleet { config }).env()?.to_string_lossy()
        );
        Ok(())
    }

    fn wrappers(config: &Config) -> Result<()> {
        for wrapper in (wrappers::Fleet { config }).effective()? {
            println!(
                ":{:<20} {}\t{}",
                wrapper.name,
                wrapper.source,
                wrapper.file.display()
            );
        }
        Ok(())
    }

    fn resolve(config: &Config, uris: &[String]) -> Result<()> {
        let profile = profile::load(&config.profile).context("unable to load runseal profile")?;
        for uri in uris {
            let path = profile::resolve(&config.profile, profile.resources.as_ref(), uri)?;
            println!("{}", path.display());
        }
        Ok(())
    }

    fn resources(config: &Config) -> Result<()> {
        let profile = profile::load(&config.profile).context("unable to load runseal profile")?;
        let root = profile::root(&config.profile, profile.resources.as_ref())?;
        println!("RUNSEAL_RESOURCE_ROOT={}", root.display());
        Ok(())
    }

    fn which(config: &Config, name: &str) -> Result<()> {
        let file = (wrappers::Fleet { config }).resolve(name)?;
        println!("{}", file.display());
        Ok(())
    }
}

struct Args;

impl Args {
    fn apply(command: &[String], injections: &[Injection]) -> Result<Vec<String>> {
        if command.is_empty() {
            bail!("command mode requires at least one command token");
        }

        let mut prefix = Vec::new();
        for injection in injections {
            let Injection::Argv(spec) = injection else {
                continue;
            };
            if !spec.enabled {
                continue;
            }
            if spec.command.trim().is_empty() {
                bail!("argv command must not be empty");
            }
            if spec.args.is_empty() {
                bail!("argv args must not be empty");
            }
            if spec.command == command[0] {
                prefix.extend(spec.args.clone());
            }
        }
        if prefix.is_empty() {
            return Ok(command.to_vec());
        }

        let mut rewritten = Vec::with_capacity(command.len() + prefix.len());
        rewritten.push(command[0].clone());
        rewritten.extend(prefix);
        rewritten.extend_from_slice(&command[1..]);
        Ok(rewritten)
    }
}

pub(crate) struct Exports;

impl Exports {
    fn map(exports: Vec<(String, String)>) -> Result<BTreeMap<String, String>> {
        let mut env = BTreeMap::new();
        for (key, value) in exports {
            if !key::valid(&key) {
                bail!("invalid exported key: {}", key);
            }
            env.insert(key, value);
        }
        Ok(env)
    }
}
