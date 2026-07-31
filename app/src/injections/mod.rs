mod env;
mod symlink;

use anyhow::{Context, Result, anyhow};

use crate::core::profile::Profile;
pub use env::Patch;
use symlink::Symlink;

pub struct Lifecycle;

impl Lifecycle {
    pub fn with<T, F>(profile: &Profile, task: F) -> Result<T>
    where
        F: FnOnce(&Patch) -> Result<T>,
    {
        let patch = Patch::build(&profile.env).context("env validation failed")?;
        let mut links = profile
            .symlink
            .iter()
            .cloned()
            .map(Symlink::new)
            .collect::<Vec<_>>();

        for link in &links {
            link.validate().context("symlink validation failed")?;
        }

        let (registered, registration) = register(&mut links);
        if let Err(error) = registration {
            let closed = shutdown(&mut links, registered);
            return match closed {
                Ok(()) => Err(error),
                Err(fault) => Err(anyhow!("{error}; also failed shutdown: {fault}")),
            };
        }

        let result = task(&patch);
        let closed = shutdown(&mut links, registered);

        match (result, closed) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(primary), Ok(())) => Err(primary),
            (Ok(_), Err(error)) => Err(error),
            (Err(primary), Err(error)) => Err(anyhow!("{primary}; also failed shutdown: {error}")),
        }
    }
}

fn register(links: &mut [Symlink]) -> (usize, Result<()>) {
    let mut registered = 0usize;
    for link in links {
        if let Err(error) = link.register() {
            return (
                registered,
                Err(error).context("symlink registration failed"),
            );
        }
        registered += 1;
    }
    (registered, Ok(()))
}

fn shutdown(links: &mut [Symlink], registered: usize) -> Result<()> {
    for at in (0..registered).rev() {
        links[at].shutdown().context("symlink shutdown failed")?;
    }
    Ok(())
}
