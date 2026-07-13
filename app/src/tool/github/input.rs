use anyhow::{Context, Result, bail};

use super::options;

pub(super) fn read(args: &[String]) -> Result<String> {
    let inline = options::optional(args, "--body");
    let file = options::optional(args, "--body-file");
    match (inline, file) {
        (Some(_), Some(_)) => bail!("pass exactly one of --body or --body-file"),
        (None, None) => bail!("pass exactly one of --body or --body-file"),
        (Some(body), None) => Ok(body),
        (None, Some(path)) => std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read body file: {path}")),
    }
}

pub(super) fn validate(args: &[String], body: &str, default: usize) -> Result<()> {
    let max = max(args, default)?;
    if max == 0 {
        return Ok(());
    }
    let count = body.chars().count();
    if count > max {
        bail!("body length {count} exceeds --body-max={max}");
    }
    Ok(())
}

fn max(args: &[String], default: usize) -> Result<usize> {
    let Some(value) = options::optional(args, "--body-max") else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .with_context(|| format!("invalid --body-max: {value}"))
}
