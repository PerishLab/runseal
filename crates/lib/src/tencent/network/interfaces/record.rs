use super::{Address, Attachment, Interface, Location, Names, response};
use crate::tencent::Failure;
use serde_json::Value;
use std::{collections::BTreeSet, net::IpAddr};

pub(super) struct Record<'a>(pub(super) &'a Value);

impl Record<'_> {
    fn optional(&self, field: &str) -> Option<&Value> {
        self.0.get(field).filter(|v| !v.is_null())
    }
    fn text(&self, field: &str) -> Result<Option<String>, Failure> {
        self.optional(field)
            .map(|v| {
                let text = v.as_str().ok_or_else(response)?;
                if text.len() > 128
                    || !text.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')
                    })
                {
                    return Err(response());
                }
                Ok(text.to_owned())
            })
            .transpose()
    }
    fn id(&self, field: &str, prefix: &str) -> Result<String, Failure> {
        let value = self.text(field)?.ok_or_else(response)?;
        Names::id(&value, prefix).map_err(|_| response())?;
        Ok(value)
    }
    fn number(&self, field: &str) -> Result<Option<u64>, Failure> {
        self.optional(field)
            .map(|v| v.as_u64().ok_or_else(response))
            .transpose()
    }
    fn flag(&self, field: &str) -> Result<Option<bool>, Failure> {
        self.optional(field)
            .map(|v| v.as_bool().ok_or_else(response))
            .transpose()
    }
    fn owner(&self) -> Result<Option<String>, Failure> {
        let owner = self.text("InstanceAccountId")?;
        if let Some(owner) = &owner
            && (owner.starts_with('0') || owner.parse::<u64>().ok().is_none_or(|n| n == 0))
        {
            return Err(response());
        }
        Ok(owner)
    }
    fn groups(&self) -> Result<Option<Vec<String>>, Failure> {
        self.optional("GroupSet")
            .map(|v| {
                let values = v
                    .as_array()
                    .filter(|v| v.len() <= 100)
                    .ok_or_else(response)?;
                let mut groups = Vec::new();
                let mut seen = BTreeSet::new();
                for value in values {
                    let id = value.as_str().ok_or_else(response)?;
                    Names::id(id, "sg-").map_err(|_| response())?;
                    if !seen.insert(id) {
                        return Err(response());
                    }
                    groups.push(id.into());
                }
                Ok(groups)
            })
            .transpose()
    }
    fn addresses(&self, ipv6: bool) -> Result<Option<Vec<Address>>, Failure> {
        let field = if ipv6 {
            "Ipv6AddressSet"
        } else {
            "PrivateIpAddressSet"
        };
        self.optional(field)
            .map(|v| {
                let values = v
                    .as_array()
                    .filter(|v| v.len() <= 1024)
                    .ok_or_else(response)?;
                let mut seen = BTreeSet::new();
                let mut addresses = Vec::new();
                for value in values {
                    let address = Record(value).address(ipv6)?;
                    if !seen.insert(address.ip) {
                        return Err(response());
                    }
                    addresses.push(address);
                }
                Ok(addresses)
            })
            .transpose()
    }
    fn address(&self, ipv6: bool) -> Result<Address, Failure> {
        let field = if ipv6 { "Address" } else { "PrivateIpAddress" };
        let ip = self
            .text(field)?
            .ok_or_else(response)?
            .parse::<IpAddr>()
            .map_err(|_| response())?;
        if ip.is_ipv6() != ipv6 || ip.is_unspecified() || ip.is_multicast() {
            return Err(response());
        }
        let public = self.text("PublicIpAddress")?;
        if let Some(value) = &public
            && !value.is_empty()
            && value.parse::<IpAddr>().is_err()
        {
            return Err(response());
        }
        let elastic = self.text("AddressId")?;
        if let Some(value) = &elastic
            && !value.is_empty()
        {
            Names::id(value, "eip-").map_err(|_| response())?;
        }
        Ok(Address {
            ip,
            primary: self.flag("Primary")?,
            public,
            elastic,
            state: self.text("State")?,
            blocked: self.flag("IsWanIpBlocked")?,
            coverage: self.coverage(&[
                "PrivateIpAddress",
                "Address",
                "Primary",
                "PublicIpAddress",
                "AddressId",
                "State",
                "IsWanIpBlocked",
                "Description",
                "QosLevel",
                "AddressType",
            ])?,
        })
    }
    pub(super) fn interface(&self, instance: &str) -> Result<Interface, Failure> {
        let attachment = Record(&self.0["Attachment"]);
        let actual = attachment.id("InstanceId", "ins-")?;
        if actual != instance {
            return Err(response());
        }
        let mut entry = Interface {
            id: self.id("NetworkInterfaceId", "eni-")?,
            location: Location {
                vpc: self.id("VpcId", "vpc-")?,
                subnet: self.id("SubnetId", "subnet-")?,
                zone: self.text("Zone")?,
            },
            attachment: Attachment {
                instance: actual,
                owner: attachment.owner()?,
                device: attachment.number("DeviceIndex")?,
                mode: self.number("AttachType")?,
            },
            primary: self.flag("Primary")?,
            kind: self.number("EniType")?,
            business: self.text("Business")?,
            state: self.text("State")?,
            status: self.text("NetworkInterfaceState")?,
            protected: self.flag("TerminationProtection")?,
            traffic: self.flag("TrafficProtection")?,
            resource: self.text("ResourceId")?,
            cdc: self.text("CdcId")?,
            groups: self.groups()?,
            ipv4: self.addresses(false)?,
            ipv6: self.addresses(true)?,
            coverage: self.coverage(&[
                "NetworkInterfaceId",
                "NetworkInterfaceName",
                "NetworkInterfaceDescription",
                "SubnetId",
                "VpcId",
                "GroupSet",
                "Primary",
                "MacAddress",
                "State",
                "NetworkInterfaceState",
                "PrivateIpAddressSet",
                "Attachment",
                "Zone",
                "CreatedTime",
                "Ipv6AddressSet",
                "TagSet",
                "EniType",
                "Business",
                "CdcId",
                "AttachType",
                "ResourceId",
                "QosLevel",
                "Ipv6Addresses",
                "Ipv6AddressCount",
                "TerminationProtection",
                "TrafficProtection",
            ])? && attachment.coverage(&[
                "InstanceId",
                "InstanceAccountId",
                "DeviceIndex",
                "AttachTime",
            ])? && self.text("CdcId")?.is_none_or(|s| s.is_empty())
                && self.number("Ipv6AddressCount")?.is_none_or(|n| n == 0)
                && self
                    .optional("Ipv6Addresses")
                    .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty)),
        };
        entry.coverage &= entry
            .ipv4
            .iter()
            .chain(&entry.ipv6)
            .flatten()
            .all(|a| a.coverage);
        Ok(entry)
    }
    fn coverage(&self, fields: &[&str]) -> Result<bool, Failure> {
        Ok(self
            .0
            .as_object()
            .ok_or_else(response)?
            .keys()
            .all(|k| fields.contains(&k.as_str())))
    }
}
