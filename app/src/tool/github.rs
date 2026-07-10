use anyhow::{Result, bail};
use serde_json::Value as JsonValue;

use super::github_support as support;
use super::github_support::{
    core_repo_contains, current_branch, current_github_repo_id, current_repo_id, github_request,
    github_request_text, optional_token, prefix_enabled, read_body, token, validate_body_max,
};

pub fn eval(command: &str, args: &[String]) -> Result<Option<String>> {
    match command {
        "issue" => issue(args),
        "pr" => pr(args),
        _ => bail!("unknown tool command: github {command}"),
    }
}

fn issue(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github issue create|comment|body ...");
    };
    match command.as_str() {
        "create" => issue_create(rest),
        "comment" => issue_comment(rest),
        "body" => issue_body(rest),
        _ => bail!("usage: runseal @tool github issue create|comment|body ..."),
    }
}

fn pr(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github pr checks probe <number>");
    };
    match command.as_str() {
        "checks" => pr_checks(rest),
        _ => bail!("usage: runseal @tool github pr checks probe <number>"),
    }
}

fn pr_checks(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github pr checks probe <number>");
    };
    match command.as_str() {
        "probe" => pr_checks_probe(rest),
        _ => bail!("usage: runseal @tool github pr checks probe <number>"),
    }
}

fn pr_checks_probe(args: &[String]) -> Result<Option<String>> {
    let [number] = args else {
        bail!("usage: runseal @tool github pr checks probe <number>");
    };
    match pr_checks_probe_http(number, args) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Ok(Some("true".to_string())),
    }
}

fn pr_checks_probe_http(number: &str, args: &[String]) -> Result<String> {
    let current_repo = current_github_repo_id()?;
    let token = optional_token(args)?;
    let pr: JsonValue = github_request(
        "GET",
        &format!("/repos/{current_repo}/pulls/{number}"),
        token.as_deref(),
        None,
    )?;
    let Some(sha) = pr
        .get("head")
        .and_then(|value| value.get("sha"))
        .and_then(JsonValue::as_str)
    else {
        bail!("GitHub API pull request payload missing head.sha");
    };
    let checks: JsonValue = github_request(
        "GET",
        &format!("/repos/{current_repo}/commits/{sha}/check-runs"),
        token.as_deref(),
        None,
    )?;
    let statuses: JsonValue = github_request(
        "GET",
        &format!("/repos/{current_repo}/commits/{sha}/status"),
        token.as_deref(),
        None,
    )?;
    let check_runs = checks
        .get("total_count")
        .and_then(JsonValue::as_u64)
        .unwrap_or(0);
    let status_count = statuses
        .get("statuses")
        .and_then(JsonValue::as_array)
        .map(|value| value.len())
        .unwrap_or(0);
    Ok(if check_runs > 0 || status_count > 0 {
        "true"
    } else {
        "false"
    }
    .to_string())
}

fn issue_comment(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github issue comment create ...");
    };
    match command.as_str() {
        "create" => issue_comment_create(rest),
        _ => bail!("usage: runseal @tool github issue comment create ..."),
    }
}

fn issue_create(args: &[String]) -> Result<Option<String>> {
    let repo = support::required_option(args, "--repo")?;
    let title = support::required_option(args, "--title")?;
    let token = token(args)?;
    let body = optional_prepared_body(args, &repo, 0)?;
    let mut payload = serde_json::Map::new();
    payload.insert("title".to_string(), serde_json::Value::String(title));
    if let Some(body) = body {
        payload.insert("body".to_string(), serde_json::Value::String(body));
    }
    github_request(
        "POST",
        &format!("/repos/{repo}/issues"),
        Some(&token),
        Some(serde_json::Value::Object(payload)),
    )
    .map(|payload| Some(serde_json::to_string(&payload).expect("GitHub payload should serialize")))
}

fn issue_body(args: &[String]) -> Result<Option<String>> {
    let [command, rest @ ..] = args else {
        bail!("usage: runseal @tool github issue body update ...");
    };
    match command.as_str() {
        "update" => issue_body_update(rest),
        _ => bail!("usage: runseal @tool github issue body update ..."),
    }
}

fn issue_comment_create(args: &[String]) -> Result<Option<String>> {
    let repo = support::required_option(args, "--repo")?;
    let number = support::required_option(args, "--number")?;
    let token = token(args)?;
    let body = prepared_body(args, &repo, 100)?;
    github_request_text(
        "POST",
        &format!("/repos/{repo}/issues/{number}/comments"),
        &token,
        body,
    )
}

fn issue_body_update(args: &[String]) -> Result<Option<String>> {
    let repo = support::required_option(args, "--repo")?;
    let number = support::required_option(args, "--number")?;
    let token = token(args)?;
    let body = prepared_body(args, &repo, 0)?;
    github_request_text(
        "PATCH",
        &format!("/repos/{repo}/issues/{number}"),
        &token,
        body,
    )
}

fn prepared_body(args: &[String], target_repo: &str, default_body_max: usize) -> Result<String> {
    let mut body = read_body(args)?;
    validate_body_max(args, &body, default_body_max)?;
    if prefix_enabled(args)? {
        body = prefix_body(target_repo, &body)?;
    }
    Ok(body)
}

fn optional_prepared_body(
    args: &[String],
    target_repo: &str,
    default_body_max: usize,
) -> Result<Option<String>> {
    let has_body = args
        .iter()
        .any(|arg| arg == "--body" || arg.starts_with("--body="));
    let has_body_file = args
        .iter()
        .any(|arg| arg == "--body-file" || arg.starts_with("--body-file="));
    match (has_body, has_body_file) {
        (false, false) => Ok(None),
        _ => prepared_body(args, target_repo, default_body_max).map(Some),
    }
}

fn prefix_body(target_repo: &str, body: &str) -> Result<String> {
    if !core_repo_contains(target_repo) {
        return Ok(body.to_string());
    }
    let current_repo = current_repo_id()?;
    if current_repo.eq_ignore_ascii_case(target_repo) {
        return Ok(body.to_string());
    }
    let branch = current_branch()?;
    let prefix = format!("Requested-By-Repo: {current_repo}\nRequested-By-Branch: {branch}\n\n");
    if body.starts_with(&prefix) {
        return Ok(body.to_string());
    }
    Ok(format!("{prefix}{body}"))
}
