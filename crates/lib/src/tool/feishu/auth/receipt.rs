use super::super::{
    Class, Error,
    fault::{Field, response},
    wire,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(crate) enum Kind {
    Exchange,
    Renew,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Return {
    pub body: String,
    pub started: i64,
    pub request: Option<String>,
    pub kind: Kind,
}

impl Drop for Return {
    fn drop(&mut self) {
        self.body.zeroize();
    }
}

impl Return {
    pub fn take(record: wire::Record, started: i64, kind: Kind) -> Result<Self, Error> {
        let mut body =
            Zeroizing::new(serde_json::to_string(&record.body.0).map_err(|_| response())?);
        if body.len() > 64 * 1024 {
            let mut error = Error::unknown(Class::Response);
            error.field = Some(Field::Size);
            error.status = Some(200);
            error.request = record.request;
            return Err(error);
        }
        Ok(Self {
            body: std::mem::take(&mut *body),
            started,
            request: record.request,
            kind,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Shape {
    pub token: Option<usize>,
    pub refresh: Option<usize>,
    pub bearer: Option<bool>,
    pub lifetime: Option<i64>,
    pub lease: Option<i64>,
    pub scopes: Option<usize>,
}

impl Return {
    pub fn data(&self) -> Result<wire::Body, Error> {
        serde_json::from_str(&self.body)
            .map(wire::Body)
            .map_err(|_| {
                let mut error = response();
                error.field = Some(Field::Json);
                error.status = Some(200);
                error.request = self.request.clone();
                error
            })
    }

    pub fn shape(&self) -> Option<Shape> {
        let body = self.data().ok()?;
        Some(Shape {
            token: body.0["access_token"].as_str().map(str::len),
            refresh: body.0["refresh_token"].as_str().map(str::len),
            bearer: body.0["token_type"].as_str().map(|s| s == "Bearer"),
            lifetime: duration(&body.0["expires_in"]),
            lease: duration(&body.0["refresh_token_expires_in"]),
            scopes: body.0["scope"]
                .as_str()
                .map(|s| s.split_whitespace().count()),
        })
    }
}

fn duration(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .filter(|n| (-1..=i64::from(i32::MAX)).contains(n))
}
