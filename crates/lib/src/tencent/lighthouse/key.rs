use super::{Failure, record::Record, response};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
pub struct Association {
    pub instance: String,
    pub user: Option<String>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Key {
    pub id: String,
    pub name: Option<String>,
    pub public: Option<String>,
    pub instances: Option<Vec<String>>,
    pub associations: Option<Vec<Association>>,
    pub created: Option<String>,
    pub complete: bool,
}

impl Record<'_> {
    pub(super) fn key(&self) -> Result<Key, Failure> {
        let public = self.text("PublicKey", 8192)?;
        if let Some(value) = public.filter(|value| !value.is_empty()) {
            ssh_key::PublicKey::from_openssh(value).map_err(|_| response())?;
        }
        let associations = self.associations()?;
        let complete = self.known(&[
            "KeyId",
            "KeyName",
            "PublicKey",
            "AssociatedInstanceIds",
            "AssociatedInstanceSet",
            "CreatedTime",
            "PrivateKey",
            "Tags",
        ]) && associations
            .as_ref()
            .is_none_or(|values| values.iter().all(|value| value.complete));
        Ok(Key {
            id: self.id("KeyId", "lhkp-")?,
            name: self.text("KeyName", 128)?.map(Into::into),
            public: public.map(Into::into),
            instances: self.ids("AssociatedInstanceIds", "lhins-")?,
            associations,
            created: self.time("CreatedTime")?,
            complete,
        })
    }

    fn associations(&self) -> Result<Option<Vec<Association>>, Failure> {
        let Some(values) = self.array("AssociatedInstanceSet", 1000)? else {
            return Ok(None);
        };
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        for value in values {
            let row = Record(value);
            let instance = row.id("InstanceId", "lhins-")?;
            let user = row.text("Username", 64)?.map(str::to_owned);
            if !seen.insert((instance.clone(), user.clone())) {
                return Err(response());
            }
            result.push(Association {
                instance,
                user,
                complete: row.known(&["InstanceId", "Username"]),
            });
        }
        Ok(Some(result))
    }
}
