use super::{Class, Failure};
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

pub(in crate::tencent) fn project<T: DeserializeOwned + Serialize>(value: &Value) -> Result<Value> {
    let request = value["RequestId"].as_str().filter(|id| {
        !id.is_empty()
            && id.len() <= 128
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    });
    let failure = || {
        let mut fault = Failure::new(
            Class::Response,
            "Tencent operation receipt incomplete or oversized; a mutation may have completed; inspect the exact resource before recovery",
        );
        fault.request = request.map(str::to_owned);
        fault
    };
    let request = request.ok_or_else(failure)?;
    let typed = T::deserialize(value).map_err(|_| failure())?;
    let mut output = serde_json::to_value(typed).map_err(|_| failure())?;
    if !bounded(&output) {
        return Err(failure().into());
    }
    let object = output.as_object_mut().ok_or_else(failure)?;
    object.insert("RequestId".into(), json!(request));
    Ok(output)
}

fn bounded(value: &Value) -> bool {
    match value {
        Value::String(text) => text.len() <= 128 * 1024,
        Value::Array(items) => items.len() <= 200 && items.iter().all(bounded),
        Value::Object(object) => object.values().all(bounded),
        _ => true,
    }
}

pub(in crate::tencent) fn text<'de, D: Deserializer<'de>>(input: D) -> Result<String, D::Error> {
    let value = String::deserialize(input)?;
    if value.is_empty() {
        return Err(serde::de::Error::custom(
            "required public identifier absent",
        ));
    }
    Ok(value)
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub(in crate::tencent) enum Number {
    Integer(u64),
    Text(#[serde(deserialize_with = "digits")] String),
}

fn digits<'de, D: Deserializer<'de>>(input: D) -> Result<String, D::Error> {
    let value = text(input)?;
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(serde::de::Error::custom("public number refused"));
    }
    Ok(value)
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub(in crate::tencent) enum Collection<T> {
    Items(Vec<T>),
    Null(()),
}
