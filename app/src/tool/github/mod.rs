use anyhow::{Result, bail};
use serde_json::Value as JsonValue;

mod git;
pub(super) mod help;
mod input;
mod options;
mod request;
mod token;

struct Checks;

struct Issue;

struct Comment;

struct Body;

pub fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "issue" => issue(args),
        "pr" => pr(args),
        _ => bail!("unknown tool command: github {command}"),
    }
}

fn issue(args: &[String]) -> Result<Option<String>> {
    Issue::eval(args)
}

impl Issue {
    fn eval(args: &[String]) -> Result<Option<String>> {
        let [command, rest @ ..] = args else {
            bail!("usage: runseal @tool github issue create|comment|body ...");
        };
        match command.as_str() {
            "create" => Self::create(rest),
            "comment" => Comment::eval(rest),
            "body" => Body::eval(rest),
            _ => bail!("usage: runseal @tool github issue create|comment|body ..."),
        }
    }

    fn create(args: &[String]) -> Result<Option<String>> {
        let repo = options::required(args, "--repo")?;
        let title = options::required(args, "--title")?;
        let token = token::required(args)?;
        let body = Body::optional(args, &repo, 0)?;
        let mut payload = serde_json::Map::new();
        payload.insert("title".to_string(), serde_json::Value::String(title));
        if let Some(body) = body {
            payload.insert("body".to_string(), serde_json::Value::String(body));
        }
        request::send(
            "POST",
            &format!("/repos/{repo}/issues"),
            Some(&token),
            Some(serde_json::Value::Object(payload)),
        )
        .map(|payload| {
            Some(serde_json::to_string(&payload).expect("GitHub payload should serialize"))
        })
    }
}

impl Comment {
    fn eval(args: &[String]) -> Result<Option<String>> {
        let [command, rest @ ..] = args else {
            bail!("usage: runseal @tool github issue comment create ...");
        };
        match command.as_str() {
            "create" => Self::create(rest),
            _ => bail!("usage: runseal @tool github issue comment create ..."),
        }
    }

    fn create(args: &[String]) -> Result<Option<String>> {
        let repo = options::required(args, "--repo")?;
        let number = options::required(args, "--number")?;
        let token = token::required(args)?;
        let body = Body::prepare(args, &repo, 100)?;
        request::text(
            "POST",
            &format!("/repos/{repo}/issues/{number}/comments"),
            &token,
            body,
        )
    }
}

impl Body {
    fn eval(args: &[String]) -> Result<Option<String>> {
        let [command, rest @ ..] = args else {
            bail!("usage: runseal @tool github issue body update ...");
        };
        match command.as_str() {
            "update" => Self::update(rest),
            _ => bail!("usage: runseal @tool github issue body update ..."),
        }
    }

    fn update(args: &[String]) -> Result<Option<String>> {
        let repo = options::required(args, "--repo")?;
        let number = options::required(args, "--number")?;
        let token = token::required(args)?;
        let body = Self::prepare(args, &repo, 0)?;
        request::text(
            "PATCH",
            &format!("/repos/{repo}/issues/{number}"),
            &token,
            body,
        )
    }

    fn prepare(args: &[String], target: &str, limit: usize) -> Result<String> {
        let mut body = input::read(args)?;
        input::validate(args, &body, limit)?;
        if options::boolean(args, "--prefix-enable")?.unwrap_or(false) {
            body = Self::prefix(target, &body)?;
        }
        Ok(body)
    }

    fn optional(args: &[String], target: &str, limit: usize) -> Result<Option<String>> {
        let inline = args
            .iter()
            .any(|arg| arg == "--body" || arg.starts_with("--body="));
        let file = args
            .iter()
            .any(|arg| arg == "--body-file" || arg.starts_with("--body-file="));
        match (inline, file) {
            (false, false) => Ok(None),
            _ => Self::prepare(args, target, limit).map(Some),
        }
    }

    fn prefix(target: &str, body: &str) -> Result<String> {
        if !git::core(target) {
            return Ok(body.to_string());
        }
        let repo = git::repo()?;
        if repo.eq_ignore_ascii_case(target) {
            return Ok(body.to_string());
        }
        let branch = git::branch()?;
        let prefix = format!("Requested-By-Repo: {repo}\nRequested-By-Branch: {branch}\n\n");
        if body.starts_with(&prefix) {
            return Ok(body.to_string());
        }
        Ok(format!("{prefix}{body}"))
    }
}

fn pr(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github pr checks probe <number>");
    };
    match command.as_str() {
        "checks" => Checks::eval(rest),
        _ => bail!("usage: runseal @tool github pr checks probe <number>"),
    }
}

impl Checks {
    fn eval(args: &[String]) -> Result<Option<String>> {
        let [command, rest @ ..] = args else {
            bail!("usage: runseal @tool github pr checks probe <number>");
        };
        match command.as_str() {
            "probe" => Self::probe(rest),
            _ => bail!("usage: runseal @tool github pr checks probe <number>"),
        }
    }

    fn probe(args: &[String]) -> Result<Option<String>> {
        let [number] = args else {
            bail!("usage: runseal @tool github pr checks probe <number>");
        };
        match Self::fetch(number, args) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(Some("true".to_string())),
        }
    }

    fn fetch(number: &str, args: &[String]) -> Result<String> {
        let repo = git::github()?;
        let token = token::optional(args)?;
        let pull: JsonValue = request::send(
            "GET",
            &format!("/repos/{repo}/pulls/{number}"),
            token.as_deref(),
            None,
        )?;
        let Some(sha) = pull
            .get("head")
            .and_then(|value| value.get("sha"))
            .and_then(JsonValue::as_str)
        else {
            bail!("GitHub API pull request payload missing head.sha");
        };
        let checks: JsonValue = request::send(
            "GET",
            &format!("/repos/{repo}/commits/{sha}/check-runs"),
            token.as_deref(),
            None,
        )?;
        let statuses: JsonValue = request::send(
            "GET",
            &format!("/repos/{repo}/commits/{sha}/status"),
            token.as_deref(),
            None,
        )?;
        let runs = checks
            .get("total_count")
            .and_then(JsonValue::as_u64)
            .unwrap_or(0);
        let count = statuses
            .get("statuses")
            .and_then(JsonValue::as_array)
            .map(|value| value.len())
            .unwrap_or(0);
        Ok(if runs > 0 || count > 0 {
            "true"
        } else {
            "false"
        }
        .to_string())
    }
}
