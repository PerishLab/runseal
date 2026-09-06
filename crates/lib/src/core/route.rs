use anyhow::{Context, Result, bail};

use super::symbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Control(Vec<String>),
    Profile {
        name: Option<String>,
        command: Vec<String>,
    },
}

impl Route {
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let Some(first) = args.first() else {
            return Ok(Self::Control(args));
        };
        let Some(name) = first.strip_prefix(':') else {
            return Ok(Self::Control(args));
        };

        let name = if name.is_empty() {
            None
        } else {
            symbol::valid(name).with_context(|| format!("invalid profile name: :{name}"))?;
            Some(name.to_string())
        };
        let command = args[1..].to_vec();
        if command.is_empty() {
            bail!("profile mode requires a command or @tool");
        }
        Ok(Self::Profile { name, command })
    }
}
