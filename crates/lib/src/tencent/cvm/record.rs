use super::{Data, Failure, Key, Operation, Request, Status, key, names::Names, price, response};
use serde_json::Value;

pub(super) struct Record<'a>(pub &'a Value);

impl Record<'_> {
    pub(super) fn text(&self, key: &str, limit: usize) -> Result<String, Failure> {
        let value = self.0[key].as_str().ok_or_else(response)?;
        Names::text(value, limit).map_err(|_| response())?;
        Ok(value.into())
    }

    pub(super) fn optional(&self, key: &str, limit: usize) -> Result<Option<String>, Failure> {
        self.0
            .get(key)
            .filter(|v| !v.is_null())
            .map(|value| {
                let value = value.as_str().ok_or_else(response)?;
                if value.is_empty() {
                    return Ok(None);
                }
                Names::text(value, limit).map_err(|_| response())?;
                Ok(Some(value.into()))
            })
            .transpose()
            .map(Option::flatten)
    }

    pub(super) fn number(&self, key: &str) -> Result<u64, Failure> {
        self.0[key].as_u64().ok_or_else(response)
    }

    pub(super) fn id(&self, key: &str, prefix: &str) -> Result<String, Failure> {
        let value = self.text(key, 64)?;
        Names::id(&value, prefix).map_err(|_| response())?;
        Ok(value)
    }

    pub(super) fn ids(&self, key: &str, prefix: &str) -> Result<Vec<String>, Failure> {
        let values = self.0[key]
            .as_array()
            .filter(|v| v.len() <= 100)
            .ok_or_else(response)?;
        let mut ids = Vec::new();
        for value in values {
            ids.push(value.as_str().ok_or_else(response)?.into());
        }
        Names::unique(&ids, prefix).map_err(|_| response())?;
        Ok(ids)
    }

    fn exact(&self, field: &str) -> Result<Option<Record<'_>>, Failure> {
        let values = self.0[field].as_array().ok_or_else(response)?;
        if values.len() > 1 || self.number("TotalCount")? != values.len() as u64 {
            return Err(response());
        }
        Ok(values.first().map(Record))
    }

    fn key(&self, expected: &str) -> Result<Key, Failure> {
        let id = self.id("KeyId", "skey-")?;
        if id != expected {
            return Err(response());
        }
        let public = self.text("PublicKey", 8192)?;
        key::public(&public).map_err(|_| response())?;
        Ok(Key {
            id,
            name: self.text("KeyName", 128)?,
            project: self.number("ProjectId")?,
            public,
            instances: self.ids("AssociatedInstanceIds", "ins-")?,
        })
    }

    fn observed(&self, expected: &str, region: &str) -> Result<Box<super::Instance>, Failure> {
        let instance = self.instance(region)?;
        if instance.id != expected {
            return Err(response());
        }
        Ok(Box::new(instance))
    }

    fn status(&self, expected: &str) -> Result<Status, Failure> {
        let id = self.id("InstanceId", "ins-")?;
        if id != expected {
            return Err(response());
        }
        Ok(Status {
            id,
            state: self.text("InstanceState", 32)?,
        })
    }
}

pub(super) fn decode(request: &Request, value: &Value) -> Result<Data, Failure> {
    let record = Record(value);
    match &request.operation {
        Operation::Quote(spec) => Ok(Data::Quote(Box::new(record.price(spec)?))),
        Operation::Quota { zone } => Ok(Data::Quota(price::quotas(
            value,
            &request.region,
            zone.as_deref(),
        )?)),
        Operation::Run { dry: true, .. } => {
            if value
                .get("InstanceIdSet")
                .filter(|v| !v.is_null())
                .is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty()))
            {
                return Err(response());
            }
            Ok(Data::Checked)
        }
        Operation::Run { .. } => {
            let ids = record.ids("InstanceIdSet", "ins-")?;
            if ids.len() != 1 {
                return Err(response());
            }
            Ok(Data::Created(ids))
        }
        Operation::Import(_) => Ok(Data::Imported(record.id("KeyId", "skey-")?)),
        Operation::Get { id } => {
            let instance = record
                .exact("InstanceSet")?
                .map(|record| record.observed(id, &request.region))
                .transpose()?;
            Ok(Data::Instance(instance))
        }
        Operation::Status { id } => {
            let status = record
                .exact("InstanceStatusSet")?
                .map(|record| record.status(id))
                .transpose()?;
            Ok(Data::Status(status))
        }
        Operation::Key { id } => Ok(Data::Key(
            record.exact("KeyPairSet")?.map(|r| r.key(id)).transpose()?,
        )),
        Operation::Rename { id, .. } | Operation::Terminate { id } | Operation::Delete { id } => {
            Ok(Data::Acknowledged(id.clone()))
        }
    }
}
