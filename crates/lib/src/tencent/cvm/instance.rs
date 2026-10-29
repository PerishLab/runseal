use super::{Failure, names::Names, record::Record, response};
use serde::Serialize;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Operating,
    Success,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub action: String,
    pub stage: Stage,
    pub request: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub id: String,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Attachment {
    pub id: String,
    pub class: String,
    pub size: u64,
    pub delete: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Facts {
    pub zone: String,
    pub vpc: String,
    pub subnet: String,
    pub project: u64,
    pub flavor: String,
    pub image: String,
    pub charge: String,
    pub disk: Attachment,
    pub disks: Vec<Attachment>,
    pub groups: Vec<String>,
    pub keys: Vec<String>,
    pub public: Option<Vec<IpAddr>>,
    pub private: Option<Vec<IpAddr>>,
    pub bandwidth: u64,
    pub billing: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub state: String,
    pub facts: Facts,
    pub progress: Option<Progress>,
    pub stop: Option<String>,
    pub protected: bool,
}

impl Record<'_> {
    pub(super) fn progress(&self) -> Result<Option<Progress>, Failure> {
        let action = self.optional("LatestOperation", 64)?;
        let stage = self.optional("LatestOperationState", 32)?;
        let request = self.optional("LatestOperationRequestId", 128)?;
        if let Some(request) = &request {
            Names::request(request).map_err(|_| response())?;
        }
        match (action, stage) {
            (None, None) if request.is_none() => Ok(None),
            (Some(action), Some(stage)) => {
                Names::lexical(&action).map_err(|_| response())?;
                let stage = match stage.as_str() {
                    "OPERATING" => Stage::Operating,
                    "SUCCESS" => Stage::Success,
                    "FAILED" => Stage::Failed,
                    _ => return Err(response()),
                };
                Ok(Some(Progress {
                    action,
                    stage,
                    request,
                }))
            }
            _ => Err(response()),
        }
    }

    pub(super) fn instance(&self, region: &str) -> Result<Instance, Failure> {
        let placement = Record(&self.0["Placement"]);
        let network = Record(&self.0["VirtualPrivateCloud"]);
        let internet = Record(&self.0["InternetAccessible"]);
        let zone = placement.text("Zone", 64)?;
        Names::zone(&zone, region).map_err(|_| response())?;
        let facts = Facts {
            zone,
            vpc: network.id("VpcId", "vpc-")?,
            subnet: network.id("SubnetId", "subnet-")?,
            project: placement.number("ProjectId")?,
            flavor: self.text("InstanceType", 64)?,
            image: self.id("ImageId", "img-")?,
            charge: self.text("InstanceChargeType", 32)?,
            disk: Record(&self.0["SystemDisk"]).disk()?,
            disks: self.disks()?,
            groups: self.ids("SecurityGroupIds", "sg-")?,
            keys: Record(&self.0["LoginSettings"]).ids("KeyIds", "skey-")?,
            public: self.addresses("PublicIpAddresses")?,
            private: self.addresses("PrivateIpAddresses")?,
            bandwidth: internet.number("InternetMaxBandwidthOut")?,
            billing: internet.text("InternetChargeType", 64)?,
        };
        Ok(Instance {
            id: self.id("InstanceId", "ins-")?,
            name: self.text("InstanceName", 128)?,
            state: self.text("InstanceState", 32)?,
            facts,
            progress: self.progress()?,
            stop: self.optional("StopChargingMode", 64)?,
            protected: self.0["DisableApiTermination"]
                .as_bool()
                .ok_or_else(response)?,
        })
    }

    fn disk(&self) -> Result<Attachment, Failure> {
        Ok(Attachment {
            id: self.id("DiskId", "disk-")?,
            class: self.text("DiskType", 64)?,
            size: self.number("DiskSize")?,
            delete: self
                .0
                .get("DeleteWithInstance")
                .filter(|v| !v.is_null())
                .map(|v| v.as_bool().ok_or_else(response))
                .transpose()?,
        })
    }

    fn disks(&self) -> Result<Vec<Attachment>, Failure> {
        let values = self.0["DataDisks"]
            .as_array()
            .filter(|v| v.len() <= 21)
            .ok_or_else(response)?;
        let mut ids = std::collections::BTreeSet::new();
        let mut disks = Vec::new();
        for value in values {
            let disk = Record(value).disk()?;
            if !ids.insert(disk.id.clone()) {
                return Err(response());
            }
            disks.push(disk);
        }
        Ok(disks)
    }

    fn addresses(&self, field: &str) -> Result<Option<Vec<IpAddr>>, Failure> {
        let Some(value) = self.0.get(field).filter(|v| !v.is_null()) else {
            return Ok(None);
        };
        let values = value
            .as_array()
            .filter(|v| v.len() <= 100)
            .ok_or_else(response)?;
        let mut addresses = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for value in values {
            let address: IpAddr = value
                .as_str()
                .ok_or_else(response)?
                .parse()
                .map_err(|_| response())?;
            if !seen.insert(address) {
                return Err(response());
            }
            addresses.push(address);
        }
        Ok(Some(addresses))
    }
}
