use super::{Failure, input, names::Names};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub name: String,
    pub project: u64,
    pub public: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Key {
    pub id: String,
    pub name: String,
    pub project: u64,
    pub public: String,
    pub instances: Vec<String>,
}

impl Import {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        Names::text(&self.name, 25)?;
        if !self
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || self.project > i64::MAX as u64
        {
            return Err(input());
        }
        public(&self.public)?;
        Ok(json!({"KeyName":self.name,"ProjectId":self.project,"PublicKey":self.public}))
    }
}

pub(super) fn public(value: &str) -> Result<(), Failure> {
    if value.len() > 8192 || value.contains(['\n', '\r']) {
        return Err(input());
    }
    let key = ssh_key::PublicKey::from_openssh(value).map_err(|_| input())?;
    let rsa = key.key_data().rsa().ok_or_else(input)?;
    let modulus = rsa.n.as_positive_bytes().ok_or_else(input)?;
    let bits = modulus.len() * 8 - modulus.first().map_or(0, |b| b.leading_zeros() as usize);
    let exponent = rsa.e.as_positive_bytes().ok_or_else(input)?;
    if !(2048..=4096).contains(&bits) || exponent.len() > 8 {
        return Err(input());
    }
    let exponent = exponent.iter().fold(0u64, |n, b| (n << 8) | u64::from(*b));
    if exponent < 3 || exponent % 2 == 0 {
        return Err(input());
    }
    Ok(())
}
