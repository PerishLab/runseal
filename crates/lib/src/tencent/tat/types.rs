use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Linux,
    Windows,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    #[serde(rename = "session-manager")]
    Session,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct Region {
    pub id: String,
    pub state: Option<Availability>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Agent {
    pub id: String,
    pub version: Option<String>,
    pub heartbeat: Option<String>,
    pub status: Option<Status>,
    pub environment: Option<Environment>,
    pub features: Option<Vec<Feature>>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Data {
    Regions {
        entries: Vec<Region>,
        total: u64,
        complete: bool,
    },
    Agent {
        id: String,
        value: Option<Agent>,
        total: u64,
        complete: bool,
    },
}

pub type Receipt = crate::tencent::Receipt<Data>;
