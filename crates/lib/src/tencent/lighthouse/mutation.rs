use super::{Data, Failure, input, names::Names, record::Record, request::Call, response};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Import {
    pub name: String,
    pub public: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Binding {
    pub key: String,
    pub instance: String,
    pub username: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Imported {
    pub id: String,
    pub fingerprint: String,
    pub complete: bool,
}

impl Import {
    pub(super) fn call(&self) -> Result<Call, Failure> {
        if self.name.is_empty() || self.name.len() > 25 {
            return Err(input());
        }
        if !self
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(input());
        }
        self.fingerprint()?;
        Ok(Call {
            action: "ImportKeyPair",
            body: json!({"KeyName":self.name,"PublicKey":self.public}),
        })
    }

    pub(super) fn fingerprint(&self) -> Result<String, Failure> {
        crate::tencent::api::key::public(&self.public)
    }

    pub(super) fn decode(&self, body: &Value) -> Result<Data, Failure> {
        let record = Record(body);
        Ok(Data::Imported(Imported {
            id: record.id("KeyId", "lhkp-")?,
            fingerprint: self.fingerprint()?,
            complete: record.known(&["KeyId", "RequestId"]),
        }))
    }
}

impl Binding {
    pub(super) fn call(&self) -> Result<Call, Failure> {
        Names::id(&self.key, "lhkp-")?;
        Names::id(&self.instance, "lhins-")?;
        let first = self.username.bytes().next().ok_or_else(input)?;
        if !(first.is_ascii_lowercase() || first == b'_') || self.username.len() > 32 {
            return Err(input());
        }
        if !self
            .username
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
        {
            return Err(input());
        }
        Ok(Call {
            action: "AssociateInstancesKeyPairs",
            body: json!({"KeyIds":[self.key],"InstanceIds":[self.instance],"AssociateType":"ONLINE","Username":self.username}),
        })
    }

    pub(super) fn decode(&self, body: &Value) -> Result<Data, Failure> {
        if !body.is_object() {
            return Err(response());
        }
        Ok(Data::Associated {
            binding: self.clone(),
            complete: Record(body).known(&["RequestId"]),
        })
    }
}
