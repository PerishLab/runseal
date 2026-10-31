use serde::Serialize;
use std::net::IpAddr;

#[derive(Clone, Debug, Serialize)]
pub struct Interface {
    pub id: String,
    pub location: Location,
    pub attachment: Attachment,
    pub primary: Option<bool>,
    pub kind: Option<u64>,
    pub business: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub protected: Option<bool>,
    pub traffic: Option<bool>,
    pub resource: Option<String>,
    pub cdc: Option<String>,
    pub groups: Option<Vec<String>>,
    pub ipv4: Option<Vec<Address>>,
    pub ipv6: Option<Vec<Address>>,
    pub coverage: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Location {
    pub vpc: String,
    pub subnet: String,
    pub zone: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Attachment {
    pub instance: String,
    pub owner: Option<String>,
    pub device: Option<u64>,
    pub mode: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Address {
    pub ip: IpAddr,
    pub primary: Option<bool>,
    pub public: Option<String>,
    pub elastic: Option<String>,
    pub state: Option<String>,
    pub blocked: Option<bool>,
    pub coverage: bool,
}
