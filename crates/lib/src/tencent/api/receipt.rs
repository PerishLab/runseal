use crate::tencent::{Identity, Outcome};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Receipt<T> {
    pub identity: Identity,
    pub service: String,
    pub version: String,
    pub action: String,
    pub region: String,
    pub observed: u64,
    pub request: String,
    pub outcome: Outcome,
    pub data: T,
}
