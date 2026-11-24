use super::{Call, Cause, Client, Error, Receipt, repository, sha};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct Listing {
    id: u64,
    attempt: u32,
    page: u32,
    size: u8,
}

impl Listing {
    pub fn new(id: u64, attempt: u32, page: u32, size: u8) -> Result<Self, Error> {
        if id == 0 || attempt == 0 || page == 0 {
            return Err(Error::local(Cause::Input));
        }
        if !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            id,
            attempt,
            page,
            size,
        })
    }

    fn confirmed(self, found: &Jobs) -> bool {
        let count = found.rows.len();
        if count > usize::from(self.size) || count as u64 > u64::from(found.total) {
            return false;
        }
        found.rows.iter().all(|row| self.job(row))
    }

    fn job(self, row: &Job) -> bool {
        if row.id == 0 || row.run != self.id || !sha(&row.head) {
            return false;
        }
        row.attempt.is_none_or(|attempt| attempt == self.attempt)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Jobs {
    #[serde(rename = "total_count")]
    pub total: u32,
    #[serde(rename = "jobs")]
    pub rows: Vec<Job>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Job {
    pub id: u64,
    #[serde(rename = "run_id")]
    pub run: u64,
    #[serde(rename = "run_attempt")]
    pub attempt: Option<u32>,
    #[serde(rename = "head_sha")]
    pub head: String,
    #[serde(rename = "html_url")]
    pub url: String,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "started_at")]
    pub started: Option<String>,
    #[serde(rename = "completed_at")]
    pub completed: Option<String>,
    pub steps: Option<Vec<Step>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Step {
    pub number: u32,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

impl Client {
    pub fn jobs(&self, owner: &str, name: &str, query: Listing) -> Result<Receipt<Jobs>, Error> {
        let repo = repository(owner, name)?;
        let path = format!(
            "/repos/{repo}/actions/runs/{}/attempts/{}/jobs?per_page={}&page={}",
            query.id, query.attempt, query.size, query.page
        );
        self.wire.send(&self.bearer, Call::Rest(&path), |value| {
            let found: Jobs = serde_json::from_value(value).ok()?;
            query.confirmed(&found).then_some(found)
        })
    }
}
