use anyhow::{Result, bail};

mod client;
mod cloudflare;
mod cloudflare_help;
mod dns_record;
mod forgejo;
mod github;
mod github_help;
mod github_support;
mod help;
mod redirect_rule;

pub fn help() -> &'static str {
    help::top()
}

pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [arg] if matches!(arg.as_str(), "-h" | "--help" | "help")) {
        print!("{}", help());
        return Ok(());
    }
    if let Some(output) = eval(args)? {
        println!("{output}");
    }
    Ok(())
}

pub fn eval(args: &[String]) -> Result<Option<String>> {
    if let Some(help) = help::progressive(args) {
        return Ok(Some(help));
    }
    match args {
        [namespace, command, rest @ ..] if namespace == "github" => github::eval(command, rest),
        [namespace, command, rest @ ..] if namespace == "forgejo" => forgejo::eval(command, rest),
        [namespace, command, rest @ ..] if namespace == "cloudflare" => {
            cloudflare::eval(command, rest)
        }
        [] => bail!("@tool requires a tool path"),
        [namespace, command, ..] => bail!("unknown tool command: {namespace} {command}"),
        [namespace] => bail!("tool namespace requires a command: {namespace}"),
    }
}
