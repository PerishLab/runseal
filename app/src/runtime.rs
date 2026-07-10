use std::{collections::BTreeMap, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};

use super::app::AppContext;
use super::config::RuntimeConfig;
use super::env_key::is_valid_env_key;
use super::internal_help;
use super::profile::{DenoProfile, InjectionProfile, Profile};
use super::{injections, profile};

mod wrapper_paths;

pub struct RunResult {
    pub exit_code: Option<i32>,
}

enum InternalCommand {
    Help(&'static str),
    Profile,
    ResolveResource(Vec<String>),
    Resources,
    Wrappers,
    WhichWrapper(String),
}

struct ResolvedCommand {
    argv: Vec<String>,
    wrapper: Option<ResolvedWrapper>,
}

struct ResolvedWrapper {
    name: String,
    file: PathBuf,
}

pub fn run(app: &dyn AppContext) -> Result<RunResult> {
    let config = app.config();
    if let Some(command) = resolve_internal_dispatch(config)? {
        run_internal(config, command)?;
        return Ok(RunResult { exit_code: Some(0) });
    }

    let profile = profile::load(&config.profile_path).context("unable to load runseal profile")?;
    let command = resolve_command(config, &profile)?;
    let run_result =
        injections::with_registered_exports(app, profile.injections.clone(), |exports| {
            let env = to_env_map(exports.to_vec())?;
            let run_exports: Vec<(String, String)> = env.into_iter().collect();
            let code = run_command(config, &profile, &command, &run_exports)?;
            Ok(RunResult {
                exit_code: Some(code),
            })
        })?;
    Ok(run_result)
}

fn resolve_internal_dispatch(config: &RuntimeConfig) -> Result<Option<InternalCommand>> {
    if config.command.is_empty() {
        bail!("command mode requires at least one command token");
    }
    let Some(name) = internal_name(&config.command[0])? else {
        return Ok(None);
    };
    Ok(Some(resolve_internal_command(&name, &config.command[1..])?))
}

fn resolve_command(config: &RuntimeConfig, profile: &Profile) -> Result<ResolvedCommand> {
    if config.command.is_empty() {
        bail!("command mode requires at least one command token");
    }
    if let Some(name) = wrapper_name(&config.command[0])? {
        let file = wrapper_paths::resolve(config, &name)?;
        let mut argv = Vec::with_capacity(config.command.len());
        argv.push(file.to_string_lossy().into_owned());
        argv.extend_from_slice(&config.command[1..]);
        return Ok(ResolvedCommand {
            argv,
            wrapper: Some(ResolvedWrapper { name, file }),
        });
    }

    Ok(ResolvedCommand {
        argv: apply_argv_injections(&config.command, &profile.injections)?,
        wrapper: None,
    })
}

fn internal_name(token: &str) -> Result<Option<String>> {
    let Some(name) = token.strip_prefix('@') else {
        return Ok(None);
    };
    if name.is_empty() {
        bail!("internal command name must not be empty");
    }
    validate_symbol_name(name)
        .with_context(|| format!("invalid internal command name: @{name}"))?;
    Ok(Some(name.to_string()))
}

fn resolve_internal_command(name: &str, args: &[String]) -> Result<InternalCommand> {
    if let Some(help) = internal_help::resolve(name, args)? {
        return Ok(InternalCommand::Help(help));
    }

    match name {
        "profile" => no_internal_args(args, "@profile").map(|()| InternalCommand::Profile),
        "resolve" => resolve(args),
        "resources" => no_internal_args(args, "@resources").map(|()| InternalCommand::Resources),
        "wrappers" => no_internal_args(args, "@wrappers").map(|()| InternalCommand::Wrappers),
        "which" => which(args),
        _ => bail!("unknown internal command: @{name}"),
    }
}

fn resolve(args: &[String]) -> Result<InternalCommand> {
    if args.is_empty() {
        bail!("@resolve requires at least one resource:// URI argument");
    }
    Ok(InternalCommand::ResolveResource(args.to_vec()))
}

fn which(args: &[String]) -> Result<InternalCommand> {
    if args.len() != 1 {
        bail!("@which requires exactly one :wrapper argument");
    }
    let Some(name) = wrapper_name(&args[0])? else {
        bail!("@which currently supports only :wrapper arguments");
    };
    Ok(InternalCommand::WhichWrapper(name))
}

fn no_internal_args(args: &[String], name: &str) -> Result<()> {
    if !args.is_empty() {
        bail!("{name} does not accept arguments");
    }
    Ok(())
}

fn wrapper_name(token: &str) -> Result<Option<String>> {
    let Some(name) = token.strip_prefix(':') else {
        return Ok(None);
    };
    if name.is_empty() {
        bail!("wrapper name must not be empty");
    }
    validate_symbol_name(name).with_context(|| format!("invalid wrapper name: :{name}"))?;
    Ok(Some(name.to_string()))
}

fn validate_symbol_name(name: &str) -> Result<()> {
    if name == "." || name == ".." {
        bail!("reserved name");
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("expected only ASCII letters, numbers, '.', '_', and '-'");
    }
    Ok(())
}

fn run_internal(config: &RuntimeConfig, command: InternalCommand) -> Result<()> {
    match command {
        InternalCommand::Help(help) => print!("{help}"),
        InternalCommand::Profile => print_profile(config)?,
        InternalCommand::ResolveResource(uris) => print_resolve_resources(config, &uris)?,
        InternalCommand::Resources => print_resources(config)?,
        InternalCommand::Wrappers => print_wrappers(config)?,
        InternalCommand::WhichWrapper(name) => print_which_wrapper(config, &name)?,
    }

    Ok(())
}

fn print_profile(config: &RuntimeConfig) -> Result<()> {
    println!("RUNSEAL_HOME={}", config.runseal_home.display());
    println!("RUNSEAL_PROFILE_HOME={}", config.profile_home.display());
    println!("RUNSEAL_PROFILE_PATH={}", config.profile_path.display());
    if let Ok(Some(resources)) = profile::load_resources(&config.profile_path)
        && let Ok(root) = profile::resolve_resource_root(&config.profile_path, Some(&resources))
    {
        println!("RUNSEAL_RESOURCE_ROOT={}", root.display());
    }
    println!(
        "RUNSEAL_WRAPPER_PATH={}",
        wrapper_paths::path_env(config)?.to_string_lossy()
    );
    Ok(())
}

fn print_wrappers(config: &RuntimeConfig) -> Result<()> {
    for wrapper in wrapper_paths::effective(config)? {
        println!(
            ":{:<20} {}\t{}",
            wrapper.name,
            wrapper.source,
            wrapper.file.display()
        );
    }
    Ok(())
}

fn print_resolve_resources(config: &RuntimeConfig, uris: &[String]) -> Result<()> {
    let profile = profile::load(&config.profile_path).context("unable to load runseal profile")?;
    for uri in uris {
        let path =
            profile::resolve_resource_uri(&config.profile_path, profile.resources.as_ref(), uri)?;
        println!("{}", path.display());
    }
    Ok(())
}

fn print_resources(config: &RuntimeConfig) -> Result<()> {
    let profile = profile::load(&config.profile_path).context("unable to load runseal profile")?;
    let root = profile::resolve_resource_root(&config.profile_path, profile.resources.as_ref())?;
    println!("RUNSEAL_RESOURCE_ROOT={}", root.display());
    Ok(())
}

fn print_which_wrapper(config: &RuntimeConfig, name: &str) -> Result<()> {
    let file = wrapper_paths::resolve(config, name)?;
    println!("{}", file.display());
    Ok(())
}

fn apply_argv_injections(
    command: &[String],
    injections: &[InjectionProfile],
) -> Result<Vec<String>> {
    if command.is_empty() {
        bail!("command mode requires at least one command token");
    }

    let mut prefix_args = Vec::new();
    for injection in injections {
        let InjectionProfile::Argv(spec) = injection else {
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
            prefix_args.extend(spec.args.clone());
        }
    }
    if prefix_args.is_empty() {
        return Ok(command.to_vec());
    }

    let mut rewritten = Vec::with_capacity(command.len() + prefix_args.len());
    rewritten.push(command[0].clone());
    rewritten.extend(prefix_args);
    rewritten.extend_from_slice(&command[1..]);
    Ok(rewritten)
}

fn to_env_map(exports: Vec<(String, String)>) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for (key, value) in exports {
        if !is_valid_env_key(&key) {
            bail!("invalid exported key: {}", key);
        }
        env.insert(key, value);
    }
    Ok(env)
}

fn run_command(
    config: &RuntimeConfig,
    profile: &Profile,
    resolved: &ResolvedCommand,
    exports: &[(String, String)],
) -> Result<i32> {
    let command = &resolved.argv;
    if command.is_empty() {
        bail!("command mode requires at least one command token");
    }
    if let Some(wrapper) = &resolved.wrapper
        && wrapper_paths::is_deno(&wrapper.file)
    {
        return run_deno_wrapper(config, profile.deno.as_ref(), resolved, exports);
    }

    let mut child = child_command(resolved);
    child.env_remove("RUNSEAL_WRAPPER_NAME");
    child.env_remove("RUNSEAL_WRAPPER_FILE");
    child.envs(run_env(config, resolved, exports)?);

    wait_for_child(child)
}

fn run_deno_wrapper(
    config: &RuntimeConfig,
    deno: Option<&DenoProfile>,
    resolved: &ResolvedCommand,
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
    child.args(expand_deno_permissions(&deno.permissions)?);
    child.arg(&wrapper.file);
    if resolved.argv.len() > 1 {
        child.args(&resolved.argv[1..]);
    }
    child.env_remove("RUNSEAL_WRAPPER_NAME");
    child.env_remove("RUNSEAL_WRAPPER_FILE");
    child.envs(run_env(config, resolved, exports)?);
    wait_for_child(child).context("failed to execute deno wrapper")
}

fn expand_deno_permissions(permissions: &[String]) -> Result<Vec<String>> {
    permissions
        .iter()
        .map(|permission| {
            shellexpand::env(permission)
                .map(|expanded| expanded.into_owned())
                .with_context(|| format!("unable to expand deno permission: {permission}"))
        })
        .collect()
}

fn wait_for_child(mut child: Command) -> Result<i32> {
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

fn run_env(
    config: &RuntimeConfig,
    resolved: &ResolvedCommand,
    exports: &[(String, String)],
) -> Result<Vec<(String, String)>> {
    let mut env = exports.to_vec();
    env.push((
        "RUNSEAL_HOME".to_string(),
        config.runseal_home.to_string_lossy().into_owned(),
    ));
    env.push((
        "RUNSEAL_PROFILE_HOME".to_string(),
        config.profile_home.to_string_lossy().into_owned(),
    ));
    env.push((
        "RUNSEAL_PROFILE_PATH".to_string(),
        config.profile_path.to_string_lossy().into_owned(),
    ));
    env.push((
        "RUNSEAL_WRAPPER_PATH".to_string(),
        wrapper_paths::path_env(config)?
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

fn child_command(resolved: &ResolvedCommand) -> Command {
    #[cfg(windows)]
    if let Some(wrapper) = &resolved.wrapper
        && wrapper_uses_cmd(&wrapper.file)
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
fn wrapper_uses_cmd(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat")
    )
}
