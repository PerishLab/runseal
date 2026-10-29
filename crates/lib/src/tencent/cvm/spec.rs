use super::{Failure, input, names::Names};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct Spec {
    pub name: String,
    pub placement: Placement,
    pub hardware: Hardware,
    pub connection: Connection,
}

#[derive(Clone, Debug, Serialize)]
pub struct Placement {
    pub zone: String,
    pub vpc: String,
    pub subnet: String,
    pub project: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hardware {
    pub flavor: String,
    pub image: String,
    pub disk: Disk,
}

#[derive(Clone, Debug, Serialize)]
pub struct Disk {
    pub class: Storage,
    pub size: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Storage {
    #[serde(rename = "CLOUD_PREMIUM")]
    Premium,
    #[serde(rename = "CLOUD_SSD")]
    Ssd,
}

impl Storage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Premium => "CLOUD_PREMIUM",
            Self::Ssd => "CLOUD_SSD",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Connection {
    pub groups: Vec<String>,
    pub key: String,
    pub public: bool,
    pub bandwidth: u32,
}

impl Spec {
    pub(super) fn body(&self, region: &str) -> Result<Value, Failure> {
        Names::text(&self.name, 60)?;
        if self.name.contains(['{', '}']) || self.placement.project > i64::MAX as u64 {
            return Err(input());
        }
        Names::zone(&self.placement.zone, region)?;
        Names::id(&self.placement.vpc, "vpc-")?;
        Names::id(&self.placement.subnet, "subnet-")?;
        Names::lexical(&self.hardware.flavor)?;
        Names::id(&self.hardware.image, "img-")?;
        Names::id(&self.connection.key, "skey-")?;
        Names::unique(&self.connection.groups, "sg-")?;
        if !(10..=4096).contains(&self.hardware.disk.size)
            || !(1..=5).contains(&self.connection.groups.len())
        {
            return Err(input());
        }
        if self.connection.bandwidth > 100
            || self.connection.public != (self.connection.bandwidth > 0)
        {
            return Err(input());
        }
        Ok(json!({
            "InstanceChargeType":"POSTPAID_BY_HOUR", "InstanceCount":1,
            "Placement":{"Zone":self.placement.zone,"ProjectId":self.placement.project},
            "InstanceType":self.hardware.flavor,"ImageId":self.hardware.image,"InstanceName":self.name,
            "SystemDisk":{"DiskType":self.hardware.disk.class.name(),"DiskSize":self.hardware.disk.size},
            "DataDisks":[],"VirtualPrivateCloud":{"VpcId":self.placement.vpc,"SubnetId":self.placement.subnet},
            "SecurityGroupIds":self.connection.groups,"LoginSettings":{"KeyIds":[self.connection.key]},
            "InternetAccessible":{"PublicIpAssigned":self.connection.public,"InternetMaxBandwidthOut":self.connection.bandwidth,"InternetChargeType":"TRAFFIC_POSTPAID_BY_HOUR"},
            "EnhancedService":{"SecurityService":{"Enabled":false},"MonitorService":{"Enabled":false},"AutomationService":{"Enabled":false}}
        }))
    }
}
