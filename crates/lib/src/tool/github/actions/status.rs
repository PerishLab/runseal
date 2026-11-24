use super::{Call, Cause, Client, Error, Filter, Origin, Receipt, repository, sha};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Error,
    Failure,
    Pending,
    Success,
}

#[derive(Clone, Debug, Serialize)]
pub struct Signal {
    state: Verdict,
    context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "target_url", skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

impl Signal {
    pub fn new(
        state: Verdict,
        context: &str,
        description: Option<&str>,
        target: Option<&str>,
    ) -> Result<Self, Error> {
        if context.trim().is_empty() || context.chars().count() > 100 {
            return Err(Error::local(Cause::Input));
        }
        if description.is_some_and(|value| value.chars().count() > 140) {
            return Err(Error::local(Cause::Input));
        }
        if target.is_some_and(|value| !link(value)) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            state,
            context: context.into(),
            description: description.map(str::to_owned),
            target: target.map(str::to_owned),
        })
    }

    fn matches(&self, found: &Status) -> bool {
        if found.id == 0 || self.state != found.state || self.context != found.context {
            return false;
        }
        self.description == found.description && self.target == found.target
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    pub id: u64,
    pub state: Verdict,
    pub context: String,
    pub description: Option<String>,
    #[serde(rename = "target_url")]
    pub target: Option<String>,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Combined {
    #[serde(rename = "sha")]
    pub head: String,
    pub repository: Origin,
    pub state: Verdict,
    #[serde(rename = "total_count")]
    pub total: u64,
    #[serde(rename = "statuses")]
    pub rows: Vec<Status>,
}

impl Client {
    pub fn statuses(
        &self,
        owner: &str,
        name: &str,
        filter: &Filter,
    ) -> Result<Receipt<Combined>, Error> {
        let repo = repository(owner, name)?;
        let path = format!(
            "/repos/{repo}/commits/{}/status?per_page={}&page={}",
            filter.head, filter.size, filter.page
        );
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Combined = serde_json::from_value(value).ok()?;
            confirmed(&repo, filter, &found).then_some(found)
        })
    }

    pub fn mark(
        &self,
        owner: &str,
        name: &str,
        head: &str,
        signal: &Signal,
    ) -> Result<Receipt<Status>, Error> {
        let repo = repository(owner, name)?;
        if !sha(head) {
            return Err(Error::local(Cause::Input));
        }
        let head = head.to_ascii_lowercase();
        let path = format!("/repos/{repo}/statuses/{head}");
        let expected = format!("https://api.github.com{path}");
        let input = serde_json::to_value(signal).map_err(|_| Error::local(Cause::Input))?;
        self.wire
            .send(&self.bearer, Call::Post(&path, input), |value| {
                let found: Status = serde_json::from_value(value).ok()?;
                (signal.matches(&found) && found.url.eq_ignore_ascii_case(&expected))
                    .then_some(found)
            })
    }
}

fn confirmed(repo: &str, filter: &Filter, found: &Combined) -> bool {
    let count = found.rows.len();
    if !found.head.eq_ignore_ascii_case(&filter.head)
        || !found.repository.name.eq_ignore_ascii_case(repo)
    {
        return false;
    }
    if count > usize::from(filter.size) || count as u64 > found.total {
        return false;
    }
    let url = format!(
        "https://api.github.com/repos/{repo}/statuses/{}",
        filter.head
    );
    found
        .rows
        .iter()
        .all(|row| row.id != 0 && row.url.eq_ignore_ascii_case(&url))
}

fn link(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return false;
    }
    url.username().is_empty() && url.password().is_none()
}
