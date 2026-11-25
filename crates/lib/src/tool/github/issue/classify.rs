use super::{Call, Cause, Client, Error, Label, Receipt, graph, valid};
use crate::github::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "variant", rename_all = "lowercase")]
pub enum Status {
    Absent,
    Issue { open: bool, kind: Option<String> },
}

#[derive(Deserialize)]
struct Fault {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    path: Vec<Value>,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "__typename")]
    class: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default, rename = "issueType")]
    kind: Option<Label>,
}

impl Client {
    pub fn classify(
        &self,
        owner: &str,
        name: &str,
        numbers: &[u32],
    ) -> Result<Receipt<BTreeMap<u32, Status>>, Error> {
        if !valid(owner, false) || !valid(name, true) {
            return Err(Error::local(Cause::Input));
        }
        if numbers.is_empty() || numbers.len() > 100 {
            return Err(Error::local(Cause::Input));
        }
        let mut unique = BTreeSet::new();
        for &number in numbers {
            if number == 0 || number > i32::MAX as u32 || !unique.insert(number) {
                return Err(Error::local(Cause::Input));
            }
        }
        let fields = unique
            .iter()
            .map(|number| {
                format!(
                    "i{number}:issueOrPullRequest(number:{number}){{__typename ... on Issue{{state issueType{{name}}}}}}"
                )
            })
            .collect::<String>();
        let query = format!(
            "query RunsealClassify($owner:String!,$name:String!){{repository(owner:$owner,name:$name){{{fields}}}}}"
        );
        let variables = json!({"owner":owner,"name":name});
        let receipt =
            self.wire
                .send(&self.bearer, Call::Raw(graph(&query, variables)), |value| {
                    Some(decode(value, &unique))
                })?;
        match receipt.value {
            Ok(value) => Ok(Receipt {
                outcome: receipt.outcome,
                evidence: receipt.evidence,
                value,
            }),
            Err(cause) => Err(Error {
                outcome: Outcome::Unknown,
                cause,
                evidence: receipt.evidence,
            }),
        }
    }
}

fn absent(value: &Value) -> Result<BTreeSet<String>, Cause> {
    let mut missing = BTreeSet::new();
    let Some(errors) = value.get("errors") else {
        return Ok(missing);
    };
    let list = errors.as_array().ok_or(Cause::Shape)?;
    for entry in list {
        let fault = Fault::deserialize(entry).map_err(|_| Cause::Shape)?;
        match (fault.kind.as_deref(), fault.path.as_slice()) {
            (Some("NOT_FOUND"), [Value::String(scope), Value::String(alias)])
                if scope == "repository" =>
            {
                missing.insert(alias.clone());
            }
            _ => return Err(Cause::Graphql),
        }
    }
    Ok(missing)
}

fn decode(value: Value, numbers: &BTreeSet<u32>) -> Result<BTreeMap<u32, Status>, Cause> {
    let missing = absent(&value)?;
    let repository = value
        .pointer("/data/repository")
        .and_then(Value::as_object)
        .ok_or(Cause::Shape)?;
    let mut observed = BTreeMap::new();
    for &number in numbers {
        let alias = format!("i{number}");
        let status = match repository.get(&alias) {
            Some(Value::Null) if missing.contains(&alias) => Status::Absent,
            Some(Value::Null) | None => return Err(Cause::Shape),
            Some(entry) => {
                let node = Node::deserialize(entry).map_err(|_| Cause::Shape)?;
                shape(node)?
            }
        };
        observed.insert(number, status);
    }
    Ok(observed)
}

fn shape(node: Node) -> Result<Status, Cause> {
    if node.class != "Issue" {
        return Ok(Status::Absent);
    }
    let state = node.state.as_deref().ok_or(Cause::Shape)?;
    if !matches!(state, "OPEN" | "CLOSED") {
        return Err(Cause::Shape);
    }
    Ok(Status::Issue {
        open: state == "OPEN",
        kind: node.kind.map(|label| label.name),
    })
}
