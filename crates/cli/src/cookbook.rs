pub fn render(entry: Option<&str>) -> anyhow::Result<&'static str> {
    match entry {
        None => Ok(include_str!("../cookbook/index.txt")),
        Some(name) => anyhow::bail!("unknown cookbook entry `{name}`; no recovery entries exist"),
    }
}
