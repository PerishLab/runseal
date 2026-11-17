use super::{check, policy, provider, role};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Request {
    Provider { name: String },
    Register(provider::Input),
    Role { name: String },
    Create(role::Input),
    Policy { name: String },
    Define(policy::Input),
    Version { name: String, version: String },
    Policies { role: String },
    Attach { role: String, policy: String },
}

pub(super) struct Plan {
    pub host: &'static str,
    pub version: &'static str,
    pub action: &'static str,
    pub fields: BTreeMap<String, String>,
}

impl Request {
    pub fn target(&self) -> super::Target {
        use super::Target;
        match self {
            Self::Provider { name } => Target::Provider { name: name.clone() },
            Self::Register(input) => Target::Provider {
                name: input.name.clone(),
            },
            Self::Role { name } => Target::Role { name: name.clone() },
            Self::Create(input) => Target::Role {
                name: input.name.clone(),
            },
            Self::Policy { name } => Target::Policy { name: name.clone() },
            Self::Define(input) => Target::Policy {
                name: input.name.clone(),
            },
            Self::Version { name, version } => Target::Version {
                name: name.clone(),
                version: version.clone(),
            },
            Self::Policies { role } => Target::Role { name: role.clone() },
            Self::Attach { role, policy } => Target::Attachment {
                role: role.clone(),
                policy: policy.clone(),
            },
        }
    }

    pub fn mutation(&self) -> bool {
        matches!(
            self,
            Self::Register(_) | Self::Create(_) | Self::Define(_) | Self::Attach { .. }
        )
    }

    pub(super) fn plan(&self) -> Result<Plan> {
        match self {
            Self::Provider { name } => {
                check::Text(name).provider()?;
                Ok(ims("GetOIDCProvider", [("OIDCProviderName", name.clone())]))
            }
            Self::Register(input) => register(input),
            Self::Role { name } => {
                check::Text(name).name(64, b".-")?;
                Ok(ram("GetRole", [("RoleName", name.clone())]))
            }
            Self::Create(input) => create(input),
            Self::Policy { name } => {
                check::Text(name).name(128, b"-")?;
                Ok(ram(
                    "GetPolicy",
                    [
                        ("PolicyName", name.clone()),
                        ("PolicyType", "Custom".into()),
                    ],
                ))
            }
            Self::Define(input) => define(input),
            Self::Version { name, version } => {
                check::Text(name).name(128, b"-")?;
                check::Text(version).version()?;
                Ok(ram(
                    "GetPolicyVersion",
                    [
                        ("PolicyName", name.clone()),
                        ("PolicyType", "Custom".into()),
                        ("VersionId", version.clone()),
                    ],
                ))
            }
            Self::Policies { role } => {
                check::Text(role).name(64, b".-")?;
                Ok(ram("ListPoliciesForRole", [("RoleName", role.clone())]))
            }
            Self::Attach { role, policy } => {
                check::Text(role).name(64, b".-")?;
                check::Text(policy).name(128, b"-")?;
                Ok(ram(
                    "AttachPolicyToRole",
                    [
                        ("RoleName", role.clone()),
                        ("PolicyName", policy.clone()),
                        ("PolicyType", "Custom".into()),
                    ],
                ))
            }
        }
    }
}

fn register(input: &provider::Input) -> Result<Plan> {
    check::Text(&input.name).provider()?;
    check::Text(&input.issuer).issuer()?;
    if !(1..=168).contains(&input.hours) {
        bail!("Alibaba OIDC issuance limit refused");
    }
    entries(&input.clients, 50, b".-_:/")?;
    entries(&input.fingerprints, 5, b"")?;
    let mut plan = ims(
        "CreateOIDCProvider",
        [
            ("OIDCProviderName", input.name.clone()),
            ("IssuerUrl", input.issuer.clone()),
            ("ClientIds", input.clients.join(",")),
            ("Fingerprints", input.fingerprints.join(",")),
            ("IssuanceLimitTime", input.hours.to_string()),
        ],
    );
    describe(&mut plan, input.description.as_deref(), 256)?;
    Ok(plan)
}

fn create(input: &role::Input) -> Result<Plan> {
    check::Text(&input.name).name(64, b".-")?;
    if !(3600..=43200).contains(&input.lifetime) {
        bail!("Alibaba role session duration refused");
    }
    let mut plan = ram(
        "CreateRole",
        [
            ("RoleName", input.name.clone()),
            ("AssumeRolePolicyDocument", check::document(&input.trust)?),
            ("MaxSessionDuration", input.lifetime.to_string()),
            ("AllowConsoleLogin", "false".into()),
        ],
    );
    describe(&mut plan, input.description.as_deref(), 1024)?;
    Ok(plan)
}

fn define(input: &policy::Input) -> Result<Plan> {
    check::Text(&input.name).name(128, b"-")?;
    let mut plan = ram(
        "CreatePolicy",
        [
            ("PolicyName", input.name.clone()),
            ("PolicyDocument", check::document(&input.document)?),
        ],
    );
    describe(&mut plan, input.description.as_deref(), 1024)?;
    Ok(plan)
}

fn describe(plan: &mut Plan, text: Option<&str>, size: usize) -> Result<()> {
    if let Some(text) = text {
        check::Text(text).description(size)?;
        plan.fields.insert("Description".into(), text.into());
    }
    Ok(())
}

fn entries(values: &[String], size: usize, extra: &[u8]) -> Result<()> {
    if values.is_empty()
        || values.len() > size
        || values.iter().collect::<BTreeSet<_>>().len() != values.len()
    {
        bail!("Alibaba OIDC list cardinality refused");
    }
    for value in values {
        check::Text(value).name(128, extra)?;
        if !value.as_bytes()[0].is_ascii_alphanumeric() {
            bail!("Alibaba OIDC list value refused");
        }
    }
    Ok(())
}

fn ims<const N: usize>(action: &'static str, fields: [(&str, String); N]) -> Plan {
    Plan {
        host: "ims.aliyuncs.com",
        version: "2019-08-15",
        action,
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}

fn ram<const N: usize>(action: &'static str, fields: [(&str, String); N]) -> Plan {
    Plan {
        host: "ram.aliyuncs.com",
        version: "2015-05-01",
        action,
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}
