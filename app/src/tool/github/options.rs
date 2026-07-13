use anyhow::{Result, bail};

pub(super) fn required(args: &[String], name: &str) -> Result<String> {
    optional(args, name).ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

pub(super) fn boolean(args: &[String], name: &str) -> Result<Option<bool>> {
    let prefix = format!("{name}=");
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == name {
            return Ok(Some(true));
        }
        if let Some(value) = arg.strip_prefix(&prefix) {
            return parse(name, value).map(Some);
        }
        index += 1;
    }
    Ok(None)
}

fn parse(name: &str, value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => bail!("{name} expects true or false"),
    }
}

pub(super) fn optional(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
        index += 1;
    }
    None
}
