use super::{Call, Cause, Client, Error, Receipt, repository};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Tags {
    pub repository: String,
    pub page: u32,
    pub rows: Vec<Tag>,
    pub next: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    pub commit: Commit,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub sha: String,
    pub url: String,
}

impl Client {
    pub fn tags(
        &self,
        owner: &str,
        name: &str,
        page: u32,
        size: u8,
    ) -> Result<Receipt<Tags>, Error> {
        let repository = repository(owner, name)?;
        if page == 0 || !(1..=100).contains(&size) {
            return Err(Error::local(Cause::Input));
        }
        let path = format!("/repos/{repository}/tags?per_page={size}&page={page}");
        self.wire
            .inspect(&self.bearer, Call::Rest(&path), |value, response| {
                let rows: Vec<Tag> = serde_json::from_value(value).ok()?;
                if rows.len() > usize::from(size) || !tags(&rows, &repository) {
                    return None;
                }
                let more = match response.headers.get("link") {
                    Some(value) => crate::github::page::continuation(value)?,
                    None => false,
                };
                let next = if more {
                    if rows.is_empty() {
                        return None;
                    }
                    Some(page.checked_add(1)?)
                } else {
                    None
                };
                Some(Tags {
                    repository: repository.clone(),
                    page,
                    rows,
                    next,
                })
            })
    }
}

fn tags(rows: &[Tag], repository: &str) -> bool {
    let mut names = BTreeSet::new();
    rows.iter().all(|row| {
        let sha = &row.commit.sha;
        let expected = format!("https://api.github.com/repos/{repository}/commits/{sha}");
        let name = !row.name.is_empty() && !row.name.chars().any(char::is_control);
        let commit = super::sha(sha) && row.commit.url.eq_ignore_ascii_case(&expected);
        name && commit && names.insert(&row.name)
    })
}
