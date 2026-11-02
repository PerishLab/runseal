use super::{
    Agent, Availability, Data, Environment, Failure, Feature, Operation, Region, Request, Status,
    names::Names, response,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn decode(request: &Request, body: &Value) -> Result<Data, Failure> {
    match &request.operation {
        Operation::Regions => Record(body).regions(),
        Operation::Lighthouse { id } => Record(body).agent(id),
    }
}

struct Record<'a>(&'a Value);

impl<'a> Record<'a> {
    fn collection(&self, key: &str, limit: usize) -> Result<&'a Vec<Value>, Failure> {
        let values = self.0[key].as_array().ok_or_else(response)?;
        let count = self.0["TotalCount"].as_u64().ok_or_else(response)?;
        if values.len() > limit || count != values.len() as u64 {
            return Err(response());
        }
        Ok(values)
    }

    fn known(&self, fields: &[&str]) -> bool {
        self.0
            .as_object()
            .is_some_and(|object| object.keys().all(|key| fields.contains(&key.as_str())))
    }

    fn text(&self, field: &str) -> Result<Option<&'a str>, Failure> {
        match self.0.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) if Names::lexical(value) => Ok(Some(value)),
            _ => Err(response()),
        }
    }

    fn regions(&self) -> Result<Data, Failure> {
        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        for value in self.collection("RegionSet", 200)? {
            let id = Record(value).text("Region")?.ok_or_else(response)?;
            Names::region(id).map_err(|_| response())?;
            if !seen.insert(id) {
                return Err(response());
            }
            let state = Record(value).text("RegionState")?.map(|label| match label {
                "AVAILABLE" => Availability::Available,
                "UNAVAILABLE" => Availability::Unavailable,
                _ => Availability::Unknown,
            });
            let complete = Record(value).known(&["Region", "RegionName", "RegionState"])
                && !matches!(state, Some(Availability::Unknown));
            entries.push(Region {
                id: id.into(),
                state,
                complete,
            });
        }
        let complete = self.known(&["RegionSet", "TotalCount", "RequestId"])
            && entries.iter().all(|entry| entry.complete);
        Ok(Data::Regions {
            total: entries.len() as u64,
            entries,
            complete,
        })
    }

    fn agent(&self, id: &str) -> Result<Data, Failure> {
        let entries = self.collection("AutomationAgentSet", 1)?;
        let value = entries
            .first()
            .map(|entry| Record(entry).facts(id))
            .transpose()?;
        let complete = self.known(&["AutomationAgentSet", "TotalCount", "RequestId"])
            && value.as_ref().is_none_or(|value| value.complete);
        Ok(Data::Agent {
            id: id.into(),
            value,
            total: entries.len() as u64,
            complete,
        })
    }

    fn facts(&self, id: &str) -> Result<Agent, Failure> {
        if self.text("InstanceId")? != Some(id) {
            return Err(response());
        }
        let status = self.text("AgentStatus")?.map(|label| match label {
            "Online" => Status::Online,
            "Offline" => Status::Offline,
            _ => Status::Unknown,
        });
        let environment = self.text("Environment")?.map(|label| match label {
            "Linux" => Environment::Linux,
            "Windows" => Environment::Windows,
            _ => Environment::Unknown,
        });
        let features = self.features()?;
        let labels = matches!(
            (&status, &environment),
            (Some(Status::Unknown), _) | (_, Some(Environment::Unknown))
        );
        let complete = self.known(&[
            "InstanceId",
            "Version",
            "LastHeartbeatTime",
            "AgentStatus",
            "Environment",
            "SupportFeatures",
        ]) && !labels
            && features
                .as_ref()
                .is_none_or(|values| !values.iter().any(|value| matches!(value, Feature::Unknown)));
        Ok(Agent {
            id: id.into(),
            version: self.text("Version")?.map(Into::into),
            heartbeat: self.heartbeat()?,
            status,
            environment,
            features,
            complete,
        })
    }

    fn heartbeat(&self) -> Result<Option<String>, Failure> {
        let value = match self.0.get("LastHeartbeatTime") {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::String(value)) if value.len() <= 64 => value,
            _ => return Err(response()),
        };
        time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
            .map_err(|_| response())?;
        Ok(Some(value.clone()))
    }

    fn features(&self) -> Result<Option<Vec<Feature>>, Failure> {
        let values = match self.0.get("SupportFeatures") {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Array(values)) if values.len() <= 100 => values,
            _ => return Err(response()),
        };
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        for value in values {
            let label = value
                .as_str()
                .filter(|label| Names::lexical(label))
                .ok_or_else(response)?;
            if !seen.insert(label) {
                return Err(response());
            }
            result.push(if label == "SESSION_MANAGER" {
                Feature::Session
            } else {
                Feature::Unknown
            });
        }
        Ok(Some(result))
    }
}
