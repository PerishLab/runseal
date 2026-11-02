use super::{Failure, Label, Platform, Stage, names::Names, record::Record, response};
use serde::Serialize;
use std::{collections::BTreeSet, net::IpAddr};

#[derive(Clone, Debug, Serialize)]
pub struct Addresses {
    pub public: Option<Vec<IpAddr>>,
    pub private: Option<Vec<IpAddr>>,
    pub ipv6: Option<Vec<IpAddr>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Login {
    pub keys: Option<Vec<String>>,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub action: Option<Label>,
    pub stage: Option<Stage>,
    pub request: Option<String>,
    pub started: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Instance {
    pub id: String,
    pub uuid: Option<String>,
    pub created: Option<String>,
    pub zone: Option<String>,
    pub state: Option<Label>,
    pub platform: Option<Platform>,
    pub addresses: Addresses,
    pub login: Option<Login>,
    pub progress: Progress,
    pub complete: bool,
}

impl Record<'_> {
    pub(super) fn instance(&self, region: &str) -> Result<Instance, Failure> {
        for key in [
            "Placement",
            "PublicIpAddresses",
            "PrivateIpAddresses",
            "VirtualPrivateCloud",
        ] {
            if self.0.get(key).is_some() {
                return Err(response());
            }
        }
        let zone = self.text("Zone", 64)?;
        if zone.is_some_and(|zone| !Names::zone(zone, region)) {
            return Err(response());
        }
        let state = self.text("InstanceState", 64)?.map(|value| {
            Label::parse(
                value,
                &[
                    "PENDING",
                    "LAUNCH_FAILED",
                    "RUNNING",
                    "STOPPED",
                    "STARTING",
                    "STOPPING",
                    "REBOOTING",
                    "SHUTDOWN",
                    "TERMINATING",
                    "DELETING",
                    "FREEZING",
                    "ENTER_RESCUE_MODE",
                    "RESCUE_MODE",
                    "EXIT_RESCUE_MODE",
                ],
            )
        });
        let platform = self.text("PlatformType", 64)?.map(|value| match value {
            "LINUX_UNIX" => Platform::Linux,
            "WINDOWS" => Platform::Windows,
            _ => Platform::Unknown,
        });
        let login = self
            .nested("LoginSettings")?
            .map(|row| {
                Ok(Login {
                    keys: row.ids("KeyIds", "lhkp-")?,
                    complete: row.known(&["KeyIds"]),
                })
            })
            .transpose()?;
        let progress = self.progress()?;
        let known = self.known(&[
            "InstanceId",
            "BundleId",
            "BlueprintId",
            "CPU",
            "Memory",
            "InstanceName",
            "InstanceChargeType",
            "SystemDisk",
            "PrivateAddresses",
            "PublicAddresses",
            "InternetAccessible",
            "RenewFlag",
            "LoginSettings",
            "InstanceState",
            "Uuid",
            "LatestOperation",
            "LatestOperationState",
            "LatestOperationRequestId",
            "LatestOperationStartedTime",
            "IsolatedTime",
            "CreatedTime",
            "ExpiredTime",
            "PlatformType",
            "Platform",
            "OsName",
            "Zone",
            "Tags",
            "InstanceRestrictState",
            "SupportIpv6Detail",
            "PublicIpv6Addresses",
            "InitInvocationId",
            "InstanceViolationDetail",
        ]);
        let coverage = [
            known,
            state.as_ref().is_none_or(Label::known),
            platform != Some(Platform::Unknown),
            login.as_ref().is_none_or(|value| value.complete),
            progress.action.as_ref().is_none_or(Label::known),
            progress.stage != Some(Stage::Unknown),
        ];
        Ok(Instance {
            id: self.id("InstanceId", "lhins-")?,
            uuid: self.text("Uuid", 128)?.map(Into::into),
            created: self.time("CreatedTime")?,
            zone: zone.map(Into::into),
            state,
            platform,
            addresses: Addresses {
                public: self.addresses("PublicAddresses", false)?,
                private: self.addresses("PrivateAddresses", false)?,
                ipv6: self.addresses("PublicIpv6Addresses", true)?,
            },
            login,
            progress,
            complete: coverage.into_iter().all(|known| known),
        })
    }

    fn progress(&self) -> Result<Progress, Failure> {
        let action = self.text("LatestOperation", 64)?.map(|value| {
            Label::parse(
                value,
                &[
                    "",
                    "CreateInstances",
                    "StartInstances",
                    "StopInstances",
                    "RebootInstances",
                    "ResetInstance",
                    "AssociateInstancesKeyPairs",
                    "DisassociateInstancesKeyPairs",
                    "ResetInstancesPassword",
                    "ModifyInstancesAttribute",
                    "TerminateInstances",
                    "DeleteInstances",
                    "IsolateInstances",
                    "RenewInstances",
                    "ApplyInstanceSnapshot",
                    "ModifyInstancesBundle",
                ],
            )
        });
        let stage = self
            .text("LatestOperationState", 64)?
            .map(|value| match value {
                "OPERATING" => Stage::Operating,
                "SUCCESS" => Stage::Success,
                "FAILED" => Stage::Failed,
                "" => Stage::Unavailable,
                _ => Stage::Unknown,
            });
        let request = self.text("LatestOperationRequestId", 128)?;
        if request.is_some_and(|value| !value.is_empty() && !Names::request(value)) {
            return Err(response());
        }
        Ok(Progress {
            action,
            stage,
            request: request.map(Into::into),
            started: self.time("LatestOperationStartedTime")?,
        })
    }

    fn addresses(&self, key: &str, ipv6: bool) -> Result<Option<Vec<IpAddr>>, Failure> {
        let Some(values) = self.array(key, 100)? else {
            return Ok(None);
        };
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        for value in values {
            let address: IpAddr = value
                .as_str()
                .ok_or_else(response)?
                .parse()
                .map_err(|_| response())?;
            if address.is_ipv6() != ipv6 || !seen.insert(address) {
                return Err(response());
            }
            result.push(address);
        }
        Ok(Some(result))
    }
}
