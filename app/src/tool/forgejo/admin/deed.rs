use anyhow::{Result, bail};

pub(super) enum Deed {
    Admin(Admin),
}

pub(super) enum Admin {
    Token(Token),
}

pub(super) enum Token {
    List {
        account: String,
    },
    Create {
        account: String,
        name: String,
        scopes: Vec<String>,
    },
    Drop {
        account: String,
        name: String,
        id: u64,
    },
}

impl Deed {
    pub(super) fn take(rest: &[String], scopes: Option<&str>) -> Result<Self> {
        let deed = match rest {
            [admin, resource, verb, account]
                if admin == "admin" && resource == "token" && verb == "list" =>
            {
                validate(account, "account")?;
                Token::List {
                    account: account.clone(),
                }
            }
            [admin, resource, verb, account, name]
                if admin == "admin" && resource == "token" && verb == "create" =>
            {
                let scopes = scopes
                    .ok_or_else(|| {
                        anyhow::anyhow!("@forgejo admin token create requires --scopes")
                    })?
                    .split(',')
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                validate(account, "account")?;
                validate(name, "token name")?;
                if scopes.is_empty() {
                    bail!("@forgejo admin --scopes cannot be empty");
                }
                for scope in &scopes {
                    validate(scope, "scope")?;
                }
                Token::Create {
                    account: account.clone(),
                    name: name.clone(),
                    scopes,
                }
            }
            [admin, resource, verb, account, name, id]
                if admin == "admin" && resource == "token" && verb == "delete" =>
            {
                validate(account, "account")?;
                validate(name, "token name")?;
                let id =
                    id.parse::<u64>().ok().filter(|id| *id > 0).ok_or_else(|| {
                        anyhow::anyhow!("@forgejo admin token ID must be positive")
                    })?;
                Token::Drop {
                    account: account.clone(),
                    name: name.clone(),
                    id,
                }
            }
            _ => bail!(
                "@forgejo admin expected token list ACCOUNT, token create ACCOUNT NAME --scopes LIST, or token delete ACCOUNT NAME ID"
            ),
        };
        Ok(Self::Admin(Admin::Token(deed)))
    }
}

fn validate(value: &str, label: &str) -> Result<()> {
    let invalid = value.is_empty()
        || value
            .chars()
            .any(|character| !character.is_ascii_alphanumeric() && !"._:-".contains(character));
    if invalid {
        bail!("@forgejo admin {label} must be one simple token");
    }
    Ok(())
}
