use serde::Serialize;

pub type Receipt = crate::tencent::Receipt<Data>;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Data {
    Entities(Page),
    Deleted { policy: u64, complete: bool },
}

#[derive(Clone, Debug, Serialize)]
pub struct Page {
    pub policy: u64,
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub next: Option<u32>,
    pub items: Vec<Entity>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entity {
    pub id: String,
    pub relation: Option<Relation>,
    pub uin: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Relation {
    User,
    Group,
    Role,
}
