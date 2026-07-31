use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use crate::core::{key, profile::Env};

#[derive(Debug, Clone)]
pub struct Patch {
    pub set: BTreeMap<String, String>,
    pub unset: BTreeSet<String>,
}

impl Patch {
    pub(super) fn build(spec: &Env) -> Result<Self> {
        for key in spec.vars.keys() {
            valid(key)?;
        }
        let mut unset = BTreeSet::new();
        for key in &spec.unset {
            valid(key)?;
            if spec.vars.contains_key(key) {
                bail!("env key cannot be both set and unset: {key}");
            }
            unset.insert(key.clone());
        }
        Ok(Self {
            set: spec.vars.clone(),
            unset,
        })
    }
}

fn valid(name: &str) -> Result<()> {
    if !key::valid(name) {
        bail!("invalid env key: {name}");
    }
    if matches!(
        name,
        "RUNSEAL_HOME" | "RUNSEAL_PROFILE" | "RUNSEAL_PROFILE_PATH" | "RUNSEAL_ROOT"
    ) {
        bail!("profile cannot override Runseal context key: {name}");
    }
    Ok(())
}
