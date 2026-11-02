use super::{Failure, names::Names};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Serialize, Serializer};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub invocation: String,
    pub command: String,
    pub instance: String,
}

impl Target {
    pub fn validate(&self) -> Result<(), Failure> {
        Names::id(&self.invocation, "inv-")?;
        Names::id(&self.command, "cmd-")?;
        Names::instance(&self.instance)
    }
}

#[derive(Clone, Debug)]
pub struct Query {
    pub target: Target,
    pub id: Option<String>,
    pub output: bool,
}

impl Query {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        self.target.validate()?;
        let mut body = json!({"Offset":0,"Limit":1,"HideOutput":!self.output});
        if let Some(id) = &self.id {
            Names::id(id, "invt-")?;
            body["InvocationTaskIds"] = json!([id]);
        } else {
            body["Filters"] = json!([{"Name":"invocation-id","Values":[self.target.invocation]},
                {"Name":"instance-id","Values":[self.target.instance]}]);
        }
        Ok(body)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Dispatch {
    pub target: Target,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct Stage(pub(super) String);

impl Stage {
    pub fn parse(value: &str) -> Self {
        let known = matches!(
            value,
            "PENDING"
                | "DELIVERING"
                | "DELIVER_DELAYED"
                | "DELIVER_FAILED"
                | "START_FAILED"
                | "RUNNING"
                | "SUCCESS"
                | "FAILED"
                | "TIMEOUT"
                | "TASK_TIMEOUT"
                | "CANCELLING"
                | "CANCELLED"
                | "TERMINATED"
        );
        Self(if known { value } else { "UNKNOWN" }.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Shell,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    User,
    Invoker,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct Interval {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Timeline {
    pub created: Option<String>,
    pub updated: Option<String>,
    pub interval: Interval,
}

#[derive(Clone, Debug, Serialize)]
pub struct Document {
    pub digest: Option<String>,
    pub kind: Option<Kind>,
    pub user: Option<String>,
    pub directory: Option<String>,
    pub timeout: Option<u64>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Execution {
    pub exit: Option<i64>,
    #[serde(serialize_with = "output")]
    pub output: Option<Vec<u8>>,
    pub dropped: Option<u64>,
    pub interval: Interval,
    pub complete: bool,
}

fn output<S: Serializer>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error> {
    value
        .as_ref()
        .map(|bytes| STANDARD.encode(bytes))
        .serialize(serializer)
}

#[derive(Clone, Debug, Serialize)]
pub struct Task {
    pub id: String,
    pub target: Target,
    pub stage: Option<Stage>,
    pub source: Option<Origin>,
    pub document: Option<Document>,
    pub result: Option<Execution>,
    pub times: Timeline,
    pub complete: bool,
}
