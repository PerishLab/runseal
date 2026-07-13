use anyhow::{Result, bail};
use serde_json::Value as JsonValue;

pub(super) mod help;
mod support;

use self::support::{Body as Input, Branch, Options, Prefix, Repo, Request, Token};

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
        let repo = Options::required(args, "--repo")?;
        let title = Options::required(args, "--title")?;
        let token = Token::required(args)?;
        let body = Body::optional(args, &repo, 0)?;
        let mut payload = serde_json::Map::new();
        payload.insert("title".to_string(), serde_json::Value::String(title));
        if let Some(body) = body {
            payload.insert("body".to_string(), serde_json::Value::String(body));
        }
        Request::send(
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
        let repo = Options::required(args, "--repo")?;
        let number = Options::required(args, "--number")?;
        let token = Token::required(args)?;
        let body = Body::prepare(args, &repo, 100)?;
        Request::text(
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
        let repo = Options::required(args, "--repo")?;
        let number = Options::required(args, "--number")?;
        let token = Token::required(args)?;
        let body = Self::prepare(args, &repo, 0)?;
        Request::text(
            "PATCH",
            &format!("/repos/{repo}/issues/{number}"),
            &token,
            body,
        )
    }

    fn prepare(args: &[String], target: &str, limit: usize) -> Result<String> {
        let mut body = Input::read(args)?;
        Input::validate(args, &body, limit)?;
        if Prefix::enabled(args)? {
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
        if !Repo::core(target) {
            return Ok(body.to_string());
        }
        let repo = Repo::current()?;
        if repo.eq_ignore_ascii_case(target) {
            return Ok(body.to_string());
        }
        let branch = Branch::current()?;
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
        let repo = Repo::github()?;
        let token = Token::optional(args)?;
        let pull: JsonValue = Request::send(
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
        let checks: JsonValue = Request::send(
            "GET",
            &format!("/repos/{repo}/commits/{sha}/check-runs"),
            token.as_deref(),
            None,
        )?;
        let statuses: JsonValue = Request::send(
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
