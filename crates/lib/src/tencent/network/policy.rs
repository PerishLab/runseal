use super::{input, names::Names};
use crate::tencent::Failure;
use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Args)]
pub struct Rules {
    #[arg(long)]
    pub version: u64,
    #[arg(long, value_parser = Rule::parse)]
    pub ingress: Vec<Rule>,
    #[arg(long, value_parser = Rule::parse)]
    pub egress: Vec<Rule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Rule {
    pub protocol: Protocol,
    pub ports: Option<String>,
    pub cidr: String,
    pub action: Action,
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Accept,
    Drop,
}

impl Rules {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        if self.version == 0 || self.ingress.len() > 100 || self.egress.len() > 100 {
            return Err(input());
        }
        let ingress = self
            .ingress
            .iter()
            .map(Rule::body)
            .collect::<Result<Vec<_>, _>>()?;
        let egress = self
            .egress
            .iter()
            .map(Rule::body)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({"Version":self.version.to_string(), "Ingress":ingress,"Egress":egress}))
    }
}

impl Rule {
    pub(super) fn body(&self) -> Result<Value, Failure> {
        Names::cidr(&self.cidr, false)?;
        let protocol = match self.protocol {
            Protocol::Tcp => "TCP",
            Protocol::Udp => "UDP",
            Protocol::Icmp => "ICMP",
            Protocol::All => "ALL",
        };
        let action = match self.action {
            Action::Accept => "ACCEPT",
            Action::Drop => "DROP",
        };
        match self.protocol {
            Protocol::Tcp | Protocol::Udp => ports(self.ports.as_deref())?,
            Protocol::Icmp | Protocol::All if self.ports.is_some() => return Err(input()),
            _ => (),
        }
        let mut body = json!({"Protocol":protocol,"CidrBlock":self.cidr,"Action":action});
        if let Some(ports) = &self.ports {
            body["Port"] = json!(if ports == "all" { "ALL" } else { ports });
        }
        if let Some(description) = &self.description {
            Names::text(description, 100)?;
            body["PolicyDescription"] = json!(description);
        }
        Ok(body)
    }
}

impl Rule {
    pub fn parse(value: &str) -> Result<Self, Failure> {
        let parts: Vec<_> = value.split(';').collect();
        if !(4..=5).contains(&parts.len()) {
            return Err(input());
        }
        let protocol = match parts[0] {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            "icmp" => Protocol::Icmp,
            "all" => Protocol::All,
            _ => return Err(input()),
        };
        let action = match parts[3] {
            "accept" => Action::Accept,
            "drop" => Action::Drop,
            _ => return Err(input()),
        };
        let rule = Self {
            protocol,
            ports: (parts[1] != "-").then(|| parts[1].into()),
            cidr: parts[2].into(),
            action,
            description: parts.get(4).map(|value| (*value).into()),
        };
        rule.body()?;
        Ok(rule)
    }
}

fn ports(value: Option<&str>) -> Result<(), Failure> {
    let value = value.ok_or_else(input)?;
    if value == "all" {
        return Ok(());
    }
    if value.len() > 128 {
        return Err(input());
    }
    for interval in value.split(',') {
        let parts: Vec<_> = interval.split('-').collect();
        if parts.is_empty() || parts.len() > 2 {
            return Err(input());
        }
        let start = port(parts[0])?;
        if parts.len() == 2 && start > port(parts[1])? {
            return Err(input());
        }
    }
    Ok(())
}

fn port(value: &str) -> Result<u16, Failure> {
    value
        .parse::<u16>()
        .ok()
        .filter(|number| *number != 0)
        .ok_or_else(input)
}
