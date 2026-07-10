mod env;
mod symlink;

use anyhow::{Context, Result, anyhow};
use std::collections::BTreeMap;

use crate::core::app::AppContext;
use crate::core::profile::{EnvProfile, InjectionProfile, SymlinkProfile};
use env::EnvInjection;
use symlink::SymlinkInjection;

pub fn execute_lifecycle(
    app: &dyn AppContext,
    specs: Vec<InjectionProfile>,
) -> Result<Vec<(String, String)>> {
    with_registered_exports(app, specs, |exports| Ok(exports.to_vec()))
}

pub fn with_registered_exports<T, F>(
    app: &dyn AppContext,
    specs: Vec<InjectionProfile>,
    work: F,
) -> Result<T>
where
    F: FnOnce(&[(String, String)]) -> Result<T>,
{
    let mut injections = build_injections(specs);

    for injection in &injections {
        injection
            .validate()
            .with_context(|| format!("{} validation failed", injection.name()))?;
    }

    let (registered, register_result) = register_injections(&mut injections);
    if let Err(register_err) = register_result {
        let shutdown_result = shutdown_registered(&mut injections, registered);
        return match shutdown_result {
            Ok(()) => Err(register_err),
            Err(shutdown_err) => Err(anyhow!(
                "{register_err}; also failed shutdown: {shutdown_err}"
            )),
        };
    }

    let work_result = run_export_and_work(app, &injections, work);
    let shutdown_result = shutdown_registered(&mut injections, registered);

    match (work_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(shutdown_err)) => Err(shutdown_err),
        (Err(primary), Err(shutdown_err)) => {
            Err(anyhow!("{primary}; also failed shutdown: {shutdown_err}"))
        }
    }
}

fn register_injections(injections: &mut [RuntimeInjection]) -> (usize, Result<()>) {
    let mut registered = 0usize;
    for injection in injections {
        if let Err(err) = injection.register() {
            return (
                registered,
                Err(err).with_context(|| format!("{} registration failed", injection.name())),
            );
        }
        registered += 1;
    }
    (registered, Ok(()))
}

fn run_export_and_work<T, F>(
    app: &dyn AppContext,
    injections: &[RuntimeInjection],
    work: F,
) -> Result<T>
where
    F: FnOnce(&[(String, String)]) -> Result<T>,
{
    let exports = collect_exports(app, injections)?;
    work(&exports)
}

fn collect_exports(
    app: &dyn AppContext,
    injections: &[RuntimeInjection],
) -> Result<Vec<(String, String)>> {
    let mut exports = Vec::new();
    let mut inherited = BTreeMap::new();
    for injection in injections {
        let exported = injection
            .export(app, &inherited)
            .with_context(|| format!("{} export failed", injection.name()))?;
        for (key, value) in &exported {
            inherited.insert(key.clone(), value.clone());
        }
        exports.extend(exported);
    }
    Ok(exports)
}

fn shutdown_registered(injections: &mut [RuntimeInjection], registered: usize) -> Result<()> {
    for idx in (0..registered).rev() {
        injections[idx]
            .shutdown()
            .with_context(|| format!("{} shutdown failed", injections[idx].name()))?;
    }
    Ok(())
}

fn build_injections(specs: Vec<InjectionProfile>) -> Vec<RuntimeInjection> {
    let mut injections = Vec::new();
    for spec in specs {
        match spec {
            InjectionProfile::Env(cfg) => push_env(&mut injections, cfg),
            InjectionProfile::Symlink(cfg) => push_symlink(&mut injections, cfg),
            InjectionProfile::Argv(_) => continue,
        }
    }
    injections
}

fn push_env(injections: &mut Vec<RuntimeInjection>, cfg: EnvProfile) {
    if cfg.enabled {
        injections.push(RuntimeInjection::Env(EnvInjection::new(cfg)));
    }
}

fn push_symlink(injections: &mut Vec<RuntimeInjection>, cfg: SymlinkProfile) {
    if cfg.enabled {
        injections.push(RuntimeInjection::Symlink(SymlinkInjection::new(cfg)));
    }
}

enum RuntimeInjection {
    Env(EnvInjection),
    Symlink(SymlinkInjection),
}

impl RuntimeInjection {
    fn name(&self) -> &'static str {
        match self {
            Self::Env(inner) => inner.name(),
            Self::Symlink(inner) => inner.name(),
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Env(inner) => inner.validate(),
            Self::Symlink(inner) => inner.validate(),
        }
    }

    fn register(&mut self) -> Result<()> {
        match self {
            Self::Env(inner) => inner.register(),
            Self::Symlink(inner) => inner.register(),
        }
    }

    fn export(
        &self,
        app: &dyn AppContext,
        _inherited: &BTreeMap<String, String>,
    ) -> Result<Vec<(String, String)>> {
        match self {
            Self::Env(inner) => inner.export(app),
            Self::Symlink(inner) => inner.export(),
        }
    }

    fn shutdown(&mut self) -> Result<()> {
        match self {
            Self::Env(inner) => inner.shutdown(),
            Self::Symlink(inner) => inner.shutdown(),
        }
    }
}
