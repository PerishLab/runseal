use super::super::{Class, Failure, Identity};
use super::query::Product;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Serialize)]
pub struct Read {
    pub identity: Identity,
    pub product: Product,
    pub service: String,
    pub action: String,
    pub version: String,
    pub region: Option<String>,
    pub observed: u64,
    pub request: String,
    pub data: Data,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Data {
    Regions(Vec<Region>),
    Zones(Vec<Region>),
    Instances(Page<Instance>),
    Shapes(Vec<Shape>),
    Images(Page<Image>),
    Blueprints(Page<Image>),
    Bundles(Page<Bundle>),
}

#[derive(Clone, Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
    pub next: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Region {
    #[serde(rename(deserialize = "Region"), alias = "Zone")]
    pub id: String,
    #[serde(rename(deserialize = "RegionName"), alias = "ZoneName")]
    pub name: String,
    #[serde(rename(deserialize = "RegionState"), alias = "ZoneState")]
    pub state: Option<String>,
    #[serde(rename(deserialize = "InstanceDisplayLabel"))]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Instance {
    #[serde(rename(deserialize = "InstanceId"))]
    pub id: String,
    #[serde(rename(deserialize = "InstanceName"))]
    pub name: Option<String>,
    #[serde(rename(deserialize = "InstanceState"))]
    pub state: String,
    #[serde(rename(deserialize = "Zone"))]
    pub zone: Option<String>,
    #[serde(rename(deserialize = "PublicIpAddresses"))]
    pub public: Option<Vec<IpAddr>>,
    #[serde(rename(deserialize = "PrivateIpAddresses"))]
    pub private: Option<Vec<IpAddr>>,
    #[serde(rename(deserialize = "Placement"), skip_serializing)]
    placement: Option<Placement>,
}

#[derive(Clone, Debug, Deserialize)]
struct Placement {
    #[serde(rename = "Zone")]
    zone: String,
}

impl Instance {
    pub(super) fn normalize(mut self) -> Result<Self, Failure> {
        if let (Some(zone), Some(placement)) = (&self.zone, &self.placement)
            && zone != &placement.zone
        {
            return Err(Failure::new(
                Class::Response,
                "Tencent instance has conflicting zone evidence",
            ));
        }
        self.zone = self
            .zone
            .or_else(|| self.placement.take().map(|item| item.zone));
        self.placement = None;
        Ok(self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shape {
    #[serde(rename(deserialize = "InstanceType"))]
    pub id: String,
    #[serde(rename(deserialize = "Zone"))]
    pub zone: String,
    #[serde(rename(deserialize = "Cpu"))]
    pub cpu: Option<u32>,
    #[serde(rename(deserialize = "Memory"))]
    pub memory: Option<u32>,
    #[serde(rename(deserialize = "Status"))]
    pub state: Option<String>,
    #[serde(rename(deserialize = "InstanceChargeType"))]
    pub charge: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Image {
    #[serde(rename(deserialize = "ImageId"))]
    pub id: String,
    #[serde(rename(deserialize = "ImageName"))]
    pub name: Option<String>,
    #[serde(rename(deserialize = "ImageState"))]
    pub state: Option<String>,
    #[serde(rename(deserialize = "Platform"))]
    pub platform: Option<String>,
    #[serde(rename(deserialize = "OsName"))]
    pub system: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bundle {
    #[serde(rename(deserialize = "BundleId"))]
    pub id: String,
    #[serde(rename(deserialize = "CPU"))]
    pub cpu: Option<u32>,
    #[serde(rename(deserialize = "Memory"))]
    pub memory: Option<u32>,
    #[serde(rename(deserialize = "SystemDiskSize"))]
    pub disk: Option<u32>,
    #[serde(rename(deserialize = "InternetMaxBandwidthOut"))]
    pub bandwidth: Option<u32>,
    #[serde(rename(deserialize = "MonthlyTraffic"))]
    pub traffic: Option<u64>,
    #[serde(rename(deserialize = "BundleSalesState"))]
    pub state: Option<String>,
    #[serde(rename(deserialize = "SupportPlatformType"))]
    pub platform: Option<String>,
}

impl Image {
    pub(super) fn blueprint(value: &serde_json::Value) -> Result<Self, Failure> {
        serde_json::from_value(serde_json::json!({
            "ImageId":value.get("BlueprintId"),
            "ImageName":value.get("BlueprintName"),
            "ImageState":value.get("BlueprintState"),
            "Platform":value.get("Platform"),
            "OsName":value.get("OsName")
        }))
        .map_err(|_| Failure::new(Class::Response, "Tencent blueprint receipt is incomplete"))
    }
}
