use anyhow::{Result, bail};

pub(super) enum Deed {
    User,
    Show(String),
    List,
    Create,
    Edit(String),
    Notes(String),
    Comment(String),
    Pull(Kind),
    Status(String),
    Branch(Kind),
    Guard(Kind),
    Repo(Kind),
    Secret(Kind),
    Flow(String),
    Run(String),
    Job(String, String),
    Task(String),
    Review(Kind),
    Label,
    Fetch(String),
}

pub(super) enum Kind {
    Show(String),
    List,
    Create,
    Edit(String),
    Merge(String),
    Drop(String),
    Set(String),
}

pub(super) fn deed(rest: &[String]) -> Result<Deed> {
    match rest {
        [kind, verb] if kind == "user" && verb == "show" => Ok(Deed::User),
        [kind, verb] if kind == "issue" && verb == "list" => Ok(Deed::List),
        [kind, verb] if kind == "issue" && verb == "create" => Ok(Deed::Create),
        [kind, verb, id] if kind == "issue" && verb == "show" => Ok(Deed::Show(id.clone())),
        [kind, verb, id] if kind == "issue" && verb == "edit" => Ok(Deed::Edit(id.clone())),
        [kind, verb, deed, id] if kind == "issue" && verb == "comment" && deed == "list" => {
            Ok(Deed::Notes(id.clone()))
        }
        [kind, verb, deed, id] if kind == "issue" && verb == "comment" && deed == "create" => {
            Ok(Deed::Comment(id.clone()))
        }
        [kind, verb] if kind == "pull" && verb == "list" => Ok(Deed::Pull(Kind::List)),
        [kind, verb] if kind == "pull" && verb == "create" => Ok(Deed::Pull(Kind::Create)),
        [kind, verb, id] if kind == "pull" && verb == "show" => {
            Ok(Deed::Pull(Kind::Show(id.clone())))
        }
        [kind, verb, id] if kind == "pull" && verb == "edit" => {
            Ok(Deed::Pull(Kind::Edit(id.clone())))
        }
        [kind, verb, id] if kind == "pull" && verb == "merge" => {
            Ok(Deed::Pull(Kind::Merge(id.clone())))
        }
        [kind, verb, id] if kind == "status" && verb == "show" => Ok(Deed::Status(id.clone())),
        [kind, verb, id] if kind == "branch" && verb == "create" => {
            Ok(Deed::Branch(Kind::Set(id.clone())))
        }
        [kind, verb, id] if kind == "branch" && verb == "show" => {
            Ok(Deed::Branch(Kind::Show(id.clone())))
        }
        [kind, verb] if kind == "protection" && verb == "create" => Ok(Deed::Guard(Kind::Create)),
        [kind, verb, id] if kind == "protection" && verb == "show" => {
            Ok(Deed::Guard(Kind::Show(id.clone())))
        }
        [kind, verb, id] if kind == "protection" && verb == "edit" => {
            Ok(Deed::Guard(Kind::Edit(id.clone())))
        }
        [kind, verb] if kind == "repo" && verb == "show" => {
            Ok(Deed::Repo(Kind::Show(String::new())))
        }
        [kind, verb] if kind == "repo" && verb == "edit" => {
            Ok(Deed::Repo(Kind::Edit(String::new())))
        }
        [kind, verb] if kind == "repo" && verb == "delete" => {
            Ok(Deed::Repo(Kind::Drop(String::new())))
        }
        [kind, verb] if kind == "secret" && verb == "list" => Ok(Deed::Secret(Kind::List)),
        [kind, verb, id] if kind == "secret" && verb == "set" => {
            Ok(Deed::Secret(Kind::Set(id.clone())))
        }
        [kind, verb, id] if kind == "secret" && verb == "delete" => {
            Ok(Deed::Secret(Kind::Drop(id.clone())))
        }
        [kind, verb, id] if kind == "workflow" && verb == "dispatch" => Ok(Deed::Flow(id.clone())),
        [kind, verb, id] if kind == "run" && verb == "show" => Ok(Deed::Run(id.clone())),
        [kind, verb, run, job] if kind == "job" && verb == "log" => {
            Ok(Deed::Job(run.clone(), job.clone()))
        }
        [kind, verb, run] if kind == "task" && verb == "list" => Ok(Deed::Task(run.clone())),
        [kind, verb, id] if kind == "review" && verb == "list" => {
            Ok(Deed::Review(Kind::Show(id.clone())))
        }
        [kind, verb, id] if kind == "review" && verb == "create" => {
            Ok(Deed::Review(Kind::Set(id.clone())))
        }
        [kind, verb] if kind == "label" && verb == "list" => Ok(Deed::Label),
        [kind, url] if kind == "get" => Ok(Deed::Fetch(url.clone())),
        _ => bail!("@forgejo expected a known resource verb"),
    }
}

pub(super) fn kind(deed: &Deed) -> &'static str {
    match deed {
        Deed::User => "user",
        Deed::Show(_) | Deed::Create | Deed::Edit(_) => "issue",
        Deed::List => "issues",
        Deed::Notes(_) => "comments",
        Deed::Comment(_) => "comment",
        Deed::Pull(Kind::List) => "pulls",
        Deed::Pull(_) => "pull",
        Deed::Status(_) => "status",
        Deed::Branch(_) => "branch",
        Deed::Guard(_) => "protection",
        Deed::Repo(_) => "repo",
        Deed::Secret(Kind::List) => "secrets",
        Deed::Secret(_) => "secret",
        Deed::Flow(_) | Deed::Run(_) => "run",
        Deed::Job(_, _) => "log",
        Deed::Task(_) => "tasks",
        Deed::Review(Kind::Show(_)) => "reviews",
        Deed::Review(_) => "review",
        Deed::Label => "labels",
        Deed::Fetch(_) => "value",
    }
}
