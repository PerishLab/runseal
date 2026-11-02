use super::{
    Document, Execution, Failure, Interval, Kind, Origin, Query, Stage, Task, Timeline,
    names::Names, record::Record, response, script,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;

pub(super) struct Reader<'a>(pub &'a Value);

impl<'a> Reader<'a> {
    pub fn decode(&self, query: &Query) -> Result<Task, Failure> {
        let record = Record(self.0);
        let target = &query.target;
        for (field, expected) in [
            ("InvocationId", &target.invocation),
            ("CommandId", &target.command),
            ("InstanceId", &target.instance),
        ] {
            if record.text(field)? != Some(expected.as_str()) {
                return Err(response());
            }
        }
        let id = record.text("InvocationTaskId")?.ok_or_else(response)?;
        Names::id(id, "invt-").map_err(|_| response())?;
        if query.id.as_deref().is_some_and(|expected| expected != id) {
            return Err(response());
        }
        let stage = self.stage()?;
        let source = record.text("InvocationSource")?.map(|value| match value {
            "USER" => Origin::User,
            "INVOKER" => Origin::Invoker,
            _ => Origin::Unknown,
        });
        let document = self
            .nested("CommandDocument")?
            .map(|reader| reader.document())
            .transpose()?;
        let result = self
            .nested("TaskResult")?
            .map(|reader| reader.result(query.output))
            .transpose()?;
        let complete = record.known(&[
            "InvocationId",
            "InvocationTaskId",
            "CommandId",
            "TaskStatus",
            "InstanceId",
            "TaskResult",
            "StartTime",
            "EndTime",
            "CreatedTime",
            "UpdatedTime",
            "CommandDocument",
            "ErrorInfo",
            "InvocationSource",
            "CommandName",
        ]);
        let coverage = [
            complete,
            stage
                .as_ref()
                .is_none_or(|value| value.value() != "UNKNOWN"),
            !matches!(source, Some(Origin::Unknown)),
            document.as_ref().is_none_or(|value| value.complete),
            result.as_ref().is_none_or(|value| value.complete),
        ];
        Ok(Task {
            id: id.into(),
            target: target.clone(),
            stage,
            source,
            document,
            result,
            times: Timeline {
                created: self.time("CreatedTime")?,
                updated: self.time("UpdatedTime")?,
                interval: Interval {
                    start: self.time("StartTime")?,
                    end: self.time("EndTime")?,
                },
            },
            complete: coverage.into_iter().all(|known| known),
        })
    }

    fn nested(&self, key: &str) -> Result<Option<Reader<'a>>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) if value.is_object() => Ok(Some(Reader(value))),
            _ => Err(response()),
        }
    }

    fn text(&self, key: &str) -> Result<Option<&'a str>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value))
                if value.len() <= 256 && !value.chars().any(char::is_control) =>
            {
                Ok(Some(value))
            }
            _ => Err(response()),
        }
    }

    fn time(&self, key: &str) -> Result<Option<String>, Failure> {
        let Some(value) = self.text(key)? else {
            return Ok(None);
        };
        time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
            .map_err(|_| response())?;
        Ok(Some(value.into()))
    }

    fn unsigned(&self, key: &str) -> Result<Option<u64>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value.as_u64().map(Some).ok_or_else(response),
        }
    }

    fn bytes(&self, key: &str, limit: usize) -> Result<Option<Vec<u8>>, Failure> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) if value.len() <= limit.div_ceil(3) * 4 => {
                let bytes = STANDARD.decode(value).map_err(|_| response())?;
                if bytes.len() > limit {
                    return Err(response());
                }
                Ok(Some(bytes))
            }
            _ => Err(response()),
        }
    }

    fn stage(&self) -> Result<Option<Stage>, Failure> {
        let Some(value) = Record(self.0).text("TaskStatus")? else {
            return Ok(None);
        };
        Ok(Some(Stage::parse(value)))
    }

    fn document(&self) -> Result<Document, Failure> {
        let record = Record(self.0);
        let kind = record.text("CommandType")?.map(|value| {
            if value == "SHELL" {
                Kind::Shell
            } else {
                Kind::Unknown
            }
        });
        let complete = record.known(&[
            "Content",
            "CommandType",
            "Timeout",
            "Username",
            "WorkingDirectory",
            "OutputCOSBucketUrl",
            "OutputCOSKeyPrefix",
        ]);
        let coverage = [
            complete,
            !matches!(kind, Some(Kind::Unknown)),
            self.empty("OutputCOSBucketUrl"),
            self.empty("OutputCOSKeyPrefix"),
        ];
        let timeout = self.unsigned("Timeout")?;
        if timeout.is_some_and(|value| !(1..=86400).contains(&value)) {
            return Err(response());
        }
        Ok(Document {
            digest: self.bytes("Content", 49152)?.as_deref().map(script::digest),
            kind,
            user: self.text("Username")?.map(Into::into),
            directory: self.text("WorkingDirectory")?.map(Into::into),
            timeout,
            complete: coverage.into_iter().all(|known| known),
        })
    }

    fn empty(&self, key: &str) -> bool {
        self.0
            .get(key)
            .is_none_or(|value| value.is_null() || value.as_str() == Some(""))
    }

    fn result(&self, output: bool) -> Result<Execution, Failure> {
        let exit = match self.0.get("ExitCode") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_i64().ok_or_else(response)?),
        };
        let complete = Record(self.0).known(&[
            "ExitCode",
            "Output",
            "ExecStartTime",
            "ExecEndTime",
            "Dropped",
            "OutputUrl",
            "OutputUploadCOSErrorInfo",
        ]) && self.empty("OutputUrl")
            && self.empty("OutputUploadCOSErrorInfo");
        Ok(Execution {
            exit,
            output: if output {
                self.bytes("Output", 24576)?
            } else {
                None
            },
            dropped: self.unsigned("Dropped")?,
            interval: Interval {
                start: self.time("ExecStartTime")?,
                end: self.time("ExecEndTime")?,
            },
            complete,
        })
    }
}
