use super::{Call, Cause, Client, Error, Receipt, reference, repository, valid};
use crate::github::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug)]
pub struct Dispatch {
    workflow: String,
    branch: String,
    inputs: BTreeMap<String, String>,
}

impl Dispatch {
    pub fn new(
        workflow: &str,
        branch: &str,
        inputs: BTreeMap<String, String>,
    ) -> Result<Self, Error> {
        if !selector(workflow) || !reference(branch) || !fields(&inputs) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            workflow: workflow.into(),
            branch: branch.into(),
            inputs,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Launch {
    #[serde(rename = "workflow_run_id")]
    pub id: u64,
    #[serde(rename = "run_url")]
    pub api: String,
    #[serde(rename = "html_url")]
    pub url: String,
}

impl Client {
    pub fn dispatch(
        &self,
        owner: &str,
        name: &str,
        request: &Dispatch,
    ) -> Result<Receipt<Launch>, Error> {
        let repo = repository(owner, name)?;
        let path = format!(
            "/repos/{repo}/actions/workflows/{}/dispatches",
            request.workflow
        );
        let body = json!({"ref":request.branch,"inputs":request.inputs});
        let mut receipt = self
            .wire
            .send(&self.bearer, Call::Dispatch(&path, body), |value| {
                let found: Launch = serde_json::from_value(value).ok()?;
                found.confirmed(&repo).then_some(found)
            })?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Launch {
    fn confirmed(&self, repo: &str) -> bool {
        if self.id == 0 {
            return false;
        }
        let api = format!(
            "https://api.github.com/repos/{repo}/actions/runs/{}",
            self.id
        );
        let web = format!("https://github.com/{repo}/actions/runs/{}", self.id);
        location(&self.api, &api) && location(&self.url, &web)
    }
}

fn location(value: &str, expected: &str) -> bool {
    Url::parse(value).is_ok() && value.eq_ignore_ascii_case(expected)
}

fn selector(value: &str) -> bool {
    if !valid(value, true) {
        return false;
    }
    value.parse::<u64>().is_ok_and(|number| number != 0)
        || value.ends_with(".yml")
        || value.ends_with(".yaml")
}

fn fields(inputs: &BTreeMap<String, String>) -> bool {
    if inputs.len() > 25 || !inputs.keys().all(|name| valid(name, false)) {
        return false;
    }
    serde_json::to_vec(inputs).is_ok_and(|value| value.len() <= 65_535)
}
