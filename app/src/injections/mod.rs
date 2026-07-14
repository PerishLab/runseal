mod env;
mod symlink;

use anyhow::{Context, Result, anyhow};

use crate::core::app::Context as App;
use crate::core::profile::Injection as Spec;
use env::Env;
use symlink::Symlink;

pub struct Lifecycle;

impl Lifecycle {
    pub fn run(app: &dyn App, specs: Vec<Spec>) -> Result<Vec<(String, String)>> {
        Self::with(app, specs, |exports| Ok(exports.to_vec()))
    }

    pub fn with<T, F>(app: &dyn App, specs: Vec<Spec>, task: F) -> Result<T>
    where
        F: FnOnce(&[(String, String)]) -> Result<T>,
    {
        let mut injections = build(specs);

        for injection in &injections {
            injection
                .validate()
                .with_context(|| format!("{} validation failed", injection.name()))?;
        }

        let (registered, registration) = register(&mut injections);
        if let Err(error) = registration {
            let closed = shutdown(&mut injections, registered);
            return match closed {
                Ok(()) => Err(error),
                Err(fault) => Err(anyhow!("{error}; also failed shutdown: {fault}")),
            };
        }

        let result = exports(app, &injections).and_then(|exports| task(&exports));
        let closed = shutdown(&mut injections, registered);

        match (result, closed) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(primary), Ok(())) => Err(primary),
            (Ok(_), Err(error)) => Err(error),
            (Err(primary), Err(error)) => Err(anyhow!("{primary}; also failed shutdown: {error}")),
        }
    }
}

fn register(injections: &mut [Injection]) -> (usize, Result<()>) {
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

fn exports(app: &dyn App, injections: &[Injection]) -> Result<Vec<(String, String)>> {
    let mut exports = Vec::new();
    for injection in injections {
        let exported = injection
            .export(app)
            .with_context(|| format!("{} export failed", injection.name()))?;
        exports.extend(exported);
    }
    Ok(exports)
}

fn shutdown(injections: &mut [Injection], registered: usize) -> Result<()> {
    for idx in (0..registered).rev() {
        injections[idx]
            .shutdown()
            .with_context(|| format!("{} shutdown failed", injections[idx].name()))?;
    }
    Ok(())
}

fn build(specs: Vec<Spec>) -> Vec<Injection> {
    let mut injections = Vec::new();
    for spec in specs {
        match spec {
            Spec::Env(cfg) if cfg.enabled => injections.push(Injection::Env(Env::new(cfg))),
            Spec::Symlink(cfg) if cfg.enabled => {
                injections.push(Injection::Symlink(Symlink::new(cfg)))
            }
            Spec::Env(_) | Spec::Symlink(_) | Spec::Argv(_) => {}
        }
    }
    injections
}

enum Injection {
    Env(Env),
    Symlink(Symlink),
}

impl Injection {
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

    fn export(&self, app: &dyn App) -> Result<Vec<(String, String)>> {
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
