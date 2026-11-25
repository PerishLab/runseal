use super::{Call, Cause, Client, Error, Receipt, reference, repository, valid};
use crate::github::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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

#[derive(Debug)]
pub struct Event {
    kind: String,
    payload: Value,
}

impl Event {
    pub fn new(kind: &str, payload: Option<Value>) -> Result<Self, Error> {
        let valid = !kind.is_empty()
            && kind.len() <= 100
            && kind.bytes().all(|byte| byte.is_ascii_graphic());
        if !valid {
            return Err(Error::local(Cause::Input));
        }
        let payload = payload.unwrap_or_else(|| json!({}));
        let Some(object) = payload.as_object() else {
            return Err(Error::local(Cause::Input));
        };
        if object.len() > 10 || serde_json::to_vec(&payload).is_err_and(|_| true) {
            return Err(Error::local(Cause::Input));
        }
        if serde_json::to_vec(&payload).is_ok_and(|bytes| bytes.len() > 65_535) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            kind: kind.into(),
            payload,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Trigger {
    pub event: String,
}

impl Client {
    pub fn trigger(
        &self,
        owner: &str,
        name: &str,
        event: &Event,
    ) -> Result<Receipt<Trigger>, Error> {
        let repo = repository(owner, name)?;
        let path = format!("/repos/{repo}/dispatches");
        let body = json!({"event_type": event.kind, "client_payload": event.payload});
        let receipt = self.wire.accept(&self.bearer, &path, body)?;
        Ok(Receipt {
            outcome: receipt.outcome,
            evidence: receipt.evidence,
            value: Trigger {
                event: event.kind.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Event;
    use crate::github::{Cause, Observation, Outcome, tests::reply, tests::serve};
    use serde_json::{Map, Value, json};
    use std::cell::RefCell;

    thread_local! {
        static EVENTS: RefCell<Vec<Observation>> = const { RefCell::new(Vec::new()) };
    }

    fn record(event: &Observation) {
        EVENTS.with(|slot| slot.borrow_mut().push(event.clone()));
    }

    fn drain() -> Vec<Observation> {
        EVENTS.with(|slot| std::mem::take(&mut *slot.borrow_mut()))
    }

    #[test]
    fn validation() {
        for bad in ["", "has space", "bad\nline", &"x".repeat(101)] {
            assert_eq!(Event::new(bad, None).unwrap_err().cause, Cause::Input);
        }
        assert_eq!(
            Event::new("follow-push", Some(json!([])))
                .unwrap_err()
                .cause,
            Cause::Input
        );
        let mut wide = Map::new();
        for index in 0..11 {
            wide.insert(format!("k{index}"), Value::from(index));
        }
        assert_eq!(
            Event::new("follow-push", Some(Value::Object(wide)))
                .unwrap_err()
                .cause,
            Cause::Input
        );
        assert_eq!(
            Event::new("follow-push", Some(json!({"big": "a".repeat(65_536)})))
                .unwrap_err()
                .cause,
            Cause::Input
        );
    }

    #[test]
    fn dispatch() {
        drain();
        let event = Event::new("follow-push", Some(json!({"secret": "private-value"}))).unwrap();
        let server = serve(vec![
            reply("204 No Content", "", ""),
            reply("200 OK", "", "{}"),
            reply("422 Unprocessable Entity", "", "bad"),
            None,
        ]);
        let client = server.client.observer(record);
        let ok = client.trigger("PerishLab", "wharf", &event).unwrap();
        assert_eq!(ok.outcome, Outcome::Accepted);
        assert_eq!(ok.evidence.status, Some(204));
        assert_eq!(ok.evidence.attempts, 1);
        assert_eq!(ok.value.event, "follow-push");

        let wrong = client.trigger("PerishLab", "wharf", &event).unwrap_err();
        assert_eq!(
            (wrong.outcome, wrong.cause),
            (Outcome::Unknown, Cause::Shape)
        );

        let refused = client.trigger("PerishLab", "wharf", &event).unwrap_err();
        assert_eq!(
            (refused.outcome, refused.cause),
            (Outcome::Refused, Cause::Http)
        );

        let dropped = client.trigger("PerishLab", "wharf", &event).unwrap_err();
        assert_eq!(
            (dropped.outcome, dropped.cause, dropped.evidence.attempts),
            (Outcome::Unknown, Cause::Network, 1)
        );

        let sent = server.thread.join().unwrap();
        assert_eq!(sent.len(), 4);
        assert!(sent[0].starts_with("POST /repos/PerishLab/wharf/dispatches HTTP/1.1"));
        assert!(sent[0].contains("\"event_type\":\"follow-push\""));

        let observed = drain();
        assert_eq!(observed.len(), 4);
        assert_eq!(observed[0].operation, "rest.post.dispatches");
        assert_eq!(observed[0].outcome, Outcome::Accepted);
        assert_eq!(observed[0].status, Some(204));
        let text = serde_json::to_string(&observed).unwrap();
        assert!(!text.contains("private-value") && !text.contains("wharf"));
    }
}
