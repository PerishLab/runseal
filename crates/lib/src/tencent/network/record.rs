use super::{
    Action, Data, Entry, Protocol, Rule,
    names::Names,
    request::{Call, Kind},
};
use crate::tencent::{Class, Failure, Page};
use serde_json::Value;

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "Tencent network response is incomplete, inconsistent or unsupported; no raw data is emitted",
    )
}

impl Call {
    pub(super) fn decode(&self, value: &Value) -> Result<Data, Failure> {
        match self.kind {
            Kind::Survey(probe) => probe.decode(&self.body, value),
            Kind::Vpc => Ok(Data::Created {
                entry: Record(&value["Vpc"]).entry("VpcSet")?,
            }),
            Kind::Subnet => Ok(Data::Created {
                entry: Record(&value["Subnet"]).entry("SubnetSet")?,
            }),
            Kind::Group => Ok(Data::Created {
                entry: Record(&value["SecurityGroup"]).entry("SecurityGroupSet")?,
            }),
            Kind::Policies => Record(&value["SecurityGroupPolicySet"]).policies(),
            Kind::Ack => Ok(Data::Ack),
            Kind::List(window, collection) => {
                let values = value[collection].as_array().ok_or_else(response)?;
                let items = values
                    .iter()
                    .map(|value| Record(value).entry(collection))
                    .collect::<Result<Vec<_>, _>>()?;
                let total = value["TotalCount"].as_u64().ok_or_else(response)?;
                let count = u32::try_from(items.len()).map_err(|_| response())?;
                let end = window.offset.checked_add(count).ok_or_else(response)?;
                if count > window.limit || (count == 0 && total > u64::from(window.offset)) {
                    return Err(response());
                }
                if count > 0 && u64::from(end) > total {
                    return Err(response());
                }
                let exact = ["VpcIds", "SubnetIds", "SecurityGroupIds"]
                    .into_iter()
                    .find_map(|key| self.body[key][0].as_str());
                if let Some(id) = exact
                    && (total > 1 || items.iter().any(|item| item.id() != id))
                {
                    return Err(response());
                }
                let unique: std::collections::BTreeSet<_> = items.iter().map(Entry::id).collect();
                if unique.len() != items.len() {
                    return Err(response());
                }
                Ok(Data::Entries {
                    page: Page {
                        items,
                        total,
                        offset: window.offset,
                        limit: window.limit,
                        next: (count > 0 && u64::from(end) < total).then_some(end),
                    },
                })
            }
        }
    }
}

struct Record<'a>(&'a Value);

impl Record<'_> {
    fn field(&self, key: &str) -> Result<String, Failure> {
        let text = self.0[key].as_str().ok_or_else(response)?;
        if text.len() > 4096 || text.chars().any(char::is_control) {
            return Err(response());
        }
        Ok(text.into())
    }

    fn entry(&self, collection: &str) -> Result<Entry, Failure> {
        let default = self
            .0
            .get("IsDefault")
            .filter(|value| !value.is_null())
            .map(|value| value.as_bool().ok_or_else(response))
            .transpose()?;
        let found = match collection {
            "VpcSet" => {
                let id = self.field("VpcId")?;
                Names::id(&id, "vpc-").map_err(|_| response())?;
                let cidr = self.field("CidrBlock")?;
                Names::cidr(&cidr, false).map_err(|_| response())?;
                Entry::Vpc {
                    id,
                    name: self.field("VpcName")?,
                    cidr,
                    default,
                }
            }
            "SubnetSet" => {
                let id = self.field("SubnetId")?;
                Names::id(&id, "subnet-").map_err(|_| response())?;
                let vpc = self.field("VpcId")?;
                Names::id(&vpc, "vpc-").map_err(|_| response())?;
                let cidr = self.field("CidrBlock")?;
                Names::cidr(&cidr, false).map_err(|_| response())?;
                Entry::Subnet {
                    id,
                    name: self.field("SubnetName")?,
                    vpc,
                    zone: self.field("Zone")?,
                    cidr,
                    default,
                }
            }
            "SecurityGroupSet" => {
                let id = self.field("SecurityGroupId")?;
                Names::id(&id, "sg-").map_err(|_| response())?;
                let project = self.field("ProjectId")?;
                project.parse::<u64>().map_err(|_| response())?;
                Entry::Group {
                    id,
                    name: self.field("SecurityGroupName")?,
                    description: self.field("SecurityGroupDesc")?,
                    project,
                    default,
                }
            }
            _ => return Err(response()),
        };
        Ok(found)
    }

    fn policies(&self) -> Result<Data, Failure> {
        let version = self.field("Version")?;
        version.parse::<u64>().map_err(|_| response())?;
        let ingress = Record(&self.0["Ingress"]).rules()?;
        let egress = Record(&self.0["Egress"]).rules()?;
        Ok(Data::Policies {
            version,
            ingress,
            egress,
        })
    }

    fn rules(&self) -> Result<Vec<Rule>, Failure> {
        let values = self.0.as_array().ok_or_else(response)?;
        if values.len() > 100 {
            return Err(response());
        }
        values.iter().map(|value| Record(value).rule()).collect()
    }

    fn rule(&self) -> Result<Rule, Failure> {
        for key in ["Ipv6CidrBlock", "SecurityGroupId"] {
            if self
                .0
                .get(key)
                .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
            {
                return Err(Failure::new(
                    Class::Unsupported,
                    "policy uses an unsupported IPv6, group or template selector; preserve the complete rule set",
                ));
            }
        }
        self.template("AddressTemplate", &["AddressId", "AddressGroupId"])?;
        self.template("ServiceTemplate", &["ServiceId", "ServiceGroupId"])?;
        let protocol = match self.field("Protocol")?.to_ascii_uppercase().as_str() {
            "TCP" => Protocol::Tcp,
            "UDP" => Protocol::Udp,
            "ICMP" => Protocol::Icmp,
            "ALL" => Protocol::All,
            _ => {
                return Err(Failure::new(
                    Class::Unsupported,
                    "policy protocol is outside the finite model",
                ));
            }
        };
        let action = match self.field("Action")?.as_str() {
            "ACCEPT" => Action::Accept,
            "DROP" => Action::Drop,
            _ => return Err(response()),
        };
        let ports = if matches!(protocol, Protocol::Tcp | Protocol::Udp) {
            let port = self.field("Port")?;
            Some(if port.eq_ignore_ascii_case("all") {
                "all".into()
            } else {
                port
            })
        } else {
            None
        };
        let rule = Rule {
            protocol,
            ports,
            cidr: self.field("CidrBlock")?,
            action,
            description: self
                .0
                .get("PolicyDescription")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
        };
        rule.body().map_err(|_| response())?;
        Ok(rule)
    }

    fn template(&self, key: &str, fields: &[&str]) -> Result<(), Failure> {
        let Some(value) = self.0.get(key).filter(|value| !value.is_null()) else {
            return Ok(());
        };
        let object = value.as_object().ok_or_else(response)?;
        if object.iter().any(|(key, value)| {
            !fields.contains(&key.as_str()) || (!value.is_null() && value.as_str() != Some(""))
        }) {
            return Err(Failure::new(
                Class::Unsupported,
                "policy uses an unsupported template selector; preserve the complete rule set",
            ));
        }
        Ok(())
    }
}
