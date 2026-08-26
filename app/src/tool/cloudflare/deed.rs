use anyhow::{Result, bail};

#[derive(Clone, Copy)]
pub(super) enum Owner {
    Account,
    User,
}

impl Owner {
    pub fn other(self) -> &'static str {
        match self {
            Self::Account => "user",
            Self::User => "account",
        }
    }
}

pub(super) enum Action {
    List,
    Show(String),
    Create,
    Edit(String),
    Roll(String),
    Drop(String),
    Verify,
    Permissions,
}

pub(super) struct Token {
    pub owner: Owner,
    pub kind: Action,
}

pub(super) enum Control {
    Worker(String),
    Domains,
    Bucket(String),
    Create(String),
    Custom(String),
    Attach(String),
    Normalize { bucket: String, domain: String },
    Detach { bucket: String, domain: String },
    Drop(String),
}

pub(super) enum Deed {
    Token(Token),
    Control(Control),
}

impl Deed {
    pub fn take(args: &[String]) -> Result<Self> {
        match args.first().map(String::as_str) {
            Some("token") => token(args).map(Self::Token),
            Some("worker") | Some("r2") => control(args).map(Self::Control),
            _ => bail!("@cloudflare expected token, worker, or r2 resource"),
        }
    }

    pub fn secret(&self) -> bool {
        matches!(
            self,
            Self::Token(Token {
                kind: Action::Create | Action::Roll(_),
                ..
            })
        )
    }

    pub fn rolls(&self) -> bool {
        matches!(
            self,
            Self::Token(Token {
                kind: Action::Roll(_),
                ..
            })
        )
    }
}

impl Token {
    pub fn label(&self) -> &'static str {
        match self.kind {
            Action::List => "tokens",
            Action::Permissions => "permissions",
            Action::Verify => "verification",
            _ => "token",
        }
    }
}

fn token(args: &[String]) -> Result<Token> {
    let [resource, owner, rest @ ..] = args else {
        bail!("@cloudflare expected token OWNER VERB");
    };
    if resource != "token" {
        bail!("@cloudflare expected token OWNER VERB");
    }
    let owner = match owner.as_str() {
        "account" => Owner::Account,
        "user" => Owner::User,
        _ => bail!("@cloudflare token owner must be account or user"),
    };
    let kind = match rest {
        [verb] if verb == "list" => Action::List,
        [verb, id] if verb == "show" => Action::Show(id.clone()),
        [verb] if verb == "create" => Action::Create,
        [verb, id] if verb == "edit" => Action::Edit(id.clone()),
        [verb, id] if verb == "roll" => Action::Roll(id.clone()),
        [verb, id] if verb == "delete" => Action::Drop(id.clone()),
        [verb] if verb == "verify" => Action::Verify,
        [resource, verb] if resource == "permission" && verb == "list" => Action::Permissions,
        _ => bail!("@cloudflare expected a known token resource verb"),
    };
    Ok(Token { owner, kind })
}

fn control(args: &[String]) -> Result<Control> {
    let line = args.iter().map(String::as_str).collect::<Vec<_>>();
    match line.as_slice() {
        ["worker", "service", "show", name] => Ok(Control::Worker((*name).into())),
        ["worker", "domain", "list"] => Ok(Control::Domains),
        ["r2", "bucket", "show", name] => Ok(Control::Bucket((*name).into())),
        ["r2", "bucket", "create", name] => Ok(Control::Create((*name).into())),
        ["r2", "bucket", "domain", "list", name] => Ok(Control::Custom((*name).into())),
        ["r2", "bucket", "domain", "create", name] => Ok(Control::Attach((*name).into())),
        ["r2", "bucket", "domain", "edit", name, hostname] => Ok(Control::Normalize {
            bucket: (*name).into(),
            domain: (*hostname).into(),
        }),
        ["r2", "bucket", "domain", "delete", name, hostname] => Ok(Control::Detach {
            bucket: (*name).into(),
            domain: (*hostname).into(),
        }),
        ["r2", "bucket", "delete", name] => Ok(Control::Drop((*name).into())),
        _ => bail!("@cloudflare expected a known worker or r2 resource verb"),
    }
}
