use super::{
    Data, Failure, Label, Schema, Window, input, names::Names, record::Record, request::Call,
    response,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub enum Operation {
    List { instance: String, window: Window },
    Create(Change),
    Delete(Change),
}

#[derive(Clone, Debug, Serialize)]
pub struct Rule {
    pub protocol: String,
    pub port: u16,
    pub cidr: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub instance: String,
    pub version: u64,
    pub rule: Rule,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub protocol: Option<Label>,
    pub ports: Option<String>,
    pub cidr: Option<String>,
    pub ipv6: Option<String>,
    pub action: Option<Label>,
    pub description: Option<String>,
    pub application: Option<String>,
    pub schema: Schema,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Page {
    pub instance: String,
    pub entries: Vec<Entry>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
    pub next: Option<u32>,
    pub version: u64,
    pub schema: Schema,
    pub complete: bool,
}

impl Operation {
    pub(super) fn call(&self) -> Result<Call, Failure> {
        match self {
            Self::List { instance, window } => {
                Names::id(instance, "lhins-")?;
                if !(1..=100).contains(&window.limit) {
                    return Err(input());
                }
                Ok(Call {
                    action: "DescribeFirewallRules",
                    body: json!({"InstanceId":instance,"Offset":window.offset,"Limit":window.limit}),
                })
            }
            Self::Create(change) => change.call("CreateFirewallRules"),
            Self::Delete(change) => {
                if change.rule.description.is_some() {
                    return Err(input());
                }
                change.call("DeleteFirewallRules")
            }
        }
    }

    pub(super) fn decode(&self, body: &Value) -> Result<Data, Failure> {
        match self {
            Self::List { instance, window } => {
                Ok(Data::Rules(Page::read(instance, *window, body)?))
            }
            Self::Create(requested) | Self::Delete(requested) => Ok(Data::Changed {
                requested: requested.clone(),
                complete: Record(body).known(&["RequestId"]),
            }),
        }
    }
}

impl Change {
    fn call(&self, action: &'static str) -> Result<Call, Failure> {
        Names::id(&self.instance, "lhins-")?;
        if self.version == 0 {
            return Err(input());
        }
        Ok(Call {
            action,
            body: json!({"InstanceId":self.instance,"FirewallVersion":self.version,"FirewallRules":[self.rule.body()?]}),
        })
    }
}

impl Rule {
    fn body(&self) -> Result<Value, Failure> {
        let cidr: ipnet::Ipv4Net = self.cidr.parse().map_err(|_| input())?;
        if self.port == 0 || cidr.addr() != cidr.network() {
            return Err(input());
        }
        if !["TCP", "UDP"].contains(&self.protocol.as_str())
            || !["ACCEPT", "DROP"].contains(&self.action.as_str())
        {
            return Err(input());
        }
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.len() > 64 || value.chars().any(char::is_control))
        {
            return Err(input());
        }
        let mut rule = json!({"Protocol":self.protocol,"Port":self.port.to_string(),"CidrBlock":self.cidr,"Action":self.action});
        if let Some(description) = &self.description {
            rule["FirewallRuleDescription"] = description.clone().into();
        }
        Ok(rule)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct Selector {
    protocol: String,
    ports: String,
    cidr: String,
    ipv6: String,
    action: String,
}

impl Page {
    fn read(instance: &str, window: Window, body: &Value) -> Result<Self, Failure> {
        let record = Record(body);
        let total = record.number("TotalCount")?;
        let version = record.number("FirewallVersion")?;
        let rows = record
            .array("FirewallRuleSet", window.limit as usize)?
            .ok_or_else(response)?;
        let end = u64::from(window.offset) + rows.len() as u64;
        if end > total || (end < total && rows.is_empty()) {
            return Err(response());
        }
        let next = if end < total {
            Some(u32::try_from(end).map_err(|_| response())?)
        } else {
            None
        };
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        for row in rows {
            let entry = Entry::read(row)?;
            if let Some(key) = entry.selector()
                && !seen.insert(key)
            {
                return Err(response());
            }
            entries.push(entry);
        }
        let schema = record.schema(&[
            "FirewallRuleSet",
            "TotalCount",
            "FirewallVersion",
            "RequestId",
        ])?;
        let complete = schema.known() && entries.iter().all(|entry| entry.complete);
        Ok(Self {
            instance: instance.into(),
            entries,
            total,
            offset: window.offset,
            limit: window.limit,
            next,
            version,
            schema,
            complete,
        })
    }
}

impl Entry {
    fn read(body: &Value) -> Result<Self, Failure> {
        let record = Record(body);
        let protocol = record
            .text("Protocol", 16)?
            .map(|value| Label::parse(value, &["TCP", "UDP", "ICMP", "ICMPv6", "ALL"]));
        let action = record
            .text("Action", 16)?
            .map(|value| Label::parse(value, &["ACCEPT", "DROP"]));
        let ports = record.text("Port", 64)?.map(String::from);
        let cidr = record.text("CidrBlock", 64)?.map(String::from);
        let ipv6 = record.text("Ipv6CidrBlock", 64)?.map(String::from);
        if ports.as_ref().is_some_and(|value| {
            value != "ALL"
                && !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || b",-".contains(&byte))
        }) {
            return Err(response());
        }
        if cidr.as_ref().is_some_and(|value| !Self::ipv4(value)) {
            return Err(response());
        }
        if ipv6.as_ref().is_some_and(|value| !Self::ipv6(value)) {
            return Err(response());
        }
        let schema = record.schema(&[
            "Protocol",
            "Port",
            "CidrBlock",
            "Ipv6CidrBlock",
            "Action",
            "FirewallRuleDescription",
            "AppType",
        ])?;
        let mut result = Self {
            protocol,
            ports,
            cidr,
            ipv6,
            action,
            description: record
                .text("FirewallRuleDescription", 256)?
                .map(String::from),
            application: record.text("AppType", 256)?.map(String::from),
            schema,
            complete: false,
        };
        result.complete = result.schema.known() && result.selector().is_some();
        Ok(result)
    }

    fn ipv4(value: &str) -> bool {
        value.is_empty()
            || value.parse::<ipnet::Ipv4Net>().is_ok()
            || value.parse::<std::net::Ipv4Addr>().is_ok()
    }

    fn ipv6(value: &str) -> bool {
        value.is_empty()
            || value.parse::<ipnet::Ipv6Net>().is_ok()
            || value.parse::<std::net::Ipv6Addr>().is_ok()
    }

    fn selector(&self) -> Option<Selector> {
        let protocol = self
            .protocol
            .as_ref()
            .filter(|value| value.known())?
            .value()
            .to_string();
        let action = self
            .action
            .as_ref()
            .filter(|value| value.known())?
            .value()
            .to_string();
        let cidr = self.cidr.clone()?;
        let ipv6 = self.ipv6.clone()?;
        match (cidr.is_empty(), ipv6.is_empty()) {
            (true, false) | (false, true) => (),
            _ => return None,
        }
        Some(Selector {
            protocol,
            action,
            ports: self.ports.clone()?,
            cidr,
            ipv6,
        })
    }
}
