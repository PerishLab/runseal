use anyhow::{Result, bail};

pub(crate) fn valid(name: &str) -> Result<()> {
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
