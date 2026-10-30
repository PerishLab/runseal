use super::{
    Data,
    names::Names,
    request::{Call, Kind},
};
use crate::tencent::{Class, Failure};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const VPC: &[&str] = &[
    "Classiclink",
    "Dcg",
    "Pcx",
    "Ip",
    "Nat",
    "Vpngw",
    "FlowLog",
    "NetworkDetect",
    "NetworkACL",
    "CVM",
    "LB",
    "CDB",
    "Cmem",
    "CTSDB",
    "MariaDB",
    "SQLServer",
    "Postgres",
    "NAS",
    "Greenplumn",
    "Ckafka",
    "Grocery",
    "HSM",
    "Tcaplus",
    "Cnas",
    "TiDB",
    "Emr",
    "SEAL",
    "CFS",
    "Oracle",
    "ElasticSearch",
    "TBaaS",
    "Itop",
    "DBAudit",
    "CynosDBPostgres",
    "Redis",
    "MongoDB",
    "DCDB",
    "CynosDBMySQL",
    "Subnet",
    "RouteTable",
];
const GROUP: &[&str] = &["CVM", "CDB", "ENI", "SG", "CLB", "TotalCount"];

#[derive(Clone, Debug, Serialize)]
pub struct Count {
    pub name: Option<String>,
    pub value: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Survey {
    pub id: String,
    pub parent: Option<String>,
    pub counts: Vec<Count>,
    pub instances: Option<Vec<Count>>,
    pub complete: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Probe {
    Vpc,
    Subnet,
    Group,
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "network survey is incomplete, inconsistent or unsupported; no raw data is emitted",
    )
}

impl Probe {
    fn shape(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Vpc => (
                "DescribeVpcResourceDashboard",
                "VpcIds",
                "ResourceDashboardSet",
            ),
            Self::Subnet => (
                "DescribeSubnetResourceDashboard",
                "SubnetIds",
                "ResourceStatisticsSet",
            ),
            Self::Group => (
                "DescribeSecurityGroupAssociationStatistics",
                "SecurityGroupIds",
                "SecurityGroupAssociationStatisticsSet",
            ),
        }
    }

    fn identity(self) -> (&'static str, &'static str) {
        match self {
            Self::Vpc => ("VpcId", "vpc-"),
            Self::Subnet => ("SubnetId", "subnet-"),
            Self::Group => ("SecurityGroupId", "sg-"),
        }
    }

    pub(super) fn call(self, id: &str) -> Result<Call, Failure> {
        Names::id(id, self.identity().1)?;
        let (action, key, _) = self.shape();
        Ok(Call {
            action,
            body: json!({key:[id]}),
            kind: Kind::Survey(self),
        })
    }

    pub(super) fn decode(self, request: &Value, value: &Value) -> Result<Data, Failure> {
        let (_, key, collection) = self.shape();
        let rows = value[collection].as_array().ok_or_else(response)?;
        if rows.len() > 1 {
            return Err(response());
        }
        let Some(row) = rows.first() else {
            return Ok(Data::Survey { entry: None });
        };
        let (field, prefix) = self.identity();
        let id = row[field].as_str().ok_or_else(response)?;
        Names::id(id, prefix).map_err(|_| response())?;
        if request[key][0].as_str() != Some(id) {
            return Err(response());
        }
        let mut survey = Survey {
            id: id.into(),
            parent: None,
            counts: Vec::new(),
            instances: None,
            complete: true,
        };
        match self {
            Self::Vpc => survey.vpc(row)?,
            Self::Subnet => survey.subnet(row)?,
            Self::Group => survey.group(row)?,
        }
        Ok(Data::Survey {
            entry: Some(survey),
        })
    }
}

fn count(value: &Value) -> Result<Option<u64>, Failure> {
    if value.is_null() {
        return Ok(None);
    }
    value.as_u64().map(Some).ok_or_else(response)
}

fn keys(value: &Value, allowed: &[&str]) -> Result<bool, Failure> {
    Ok(value
        .as_object()
        .ok_or_else(response)?
        .keys()
        .all(|key| allowed.contains(&key.as_str())))
}

impl Survey {
    fn counts(&mut self, row: &Value, fields: &[&str]) -> Result<(), Failure> {
        for name in fields {
            let value = count(&row[*name])?;
            self.complete &= value.is_some();
            self.counts.push(Count {
                name: Some((*name).into()),
                value,
            });
        }
        Ok(())
    }

    fn vpc(&mut self, row: &Value) -> Result<(), Failure> {
        let mut fields = VPC.to_vec();
        fields.extend(["VpcId", "SubnetId"]);
        self.complete &= keys(row, &fields)?;
        if let Some(subnet) = row.get("SubnetId").filter(|v| !v.is_null())
            && subnet.as_str() != Some("")
        {
            return Err(response());
        }
        self.counts(row, VPC)
    }

    fn subnet(&mut self, row: &Value) -> Result<(), Failure> {
        self.complete &= keys(
            row,
            &["VpcId", "SubnetId", "Ip", "ResourceStatisticsItemSet"],
        )?;
        if let Some(parent) = row.get("VpcId").filter(|v| !v.is_null()) {
            let parent = parent.as_str().ok_or_else(response)?;
            Names::id(parent, "vpc-").map_err(|_| response())?;
            self.parent = Some(parent.into());
        }
        self.complete &= self.parent.is_some();
        self.counts(row, &["Ip"])?;
        self.instances(row.get("ResourceStatisticsItemSet"), false)
    }

    fn group(&mut self, row: &Value) -> Result<(), Failure> {
        let mut fields = GROUP.to_vec();
        fields.extend(["SecurityGroupId", "InstanceStatistics"]);
        self.complete &= keys(row, &fields)?;
        self.counts(row, GROUP)?;
        self.instances(row.get("InstanceStatistics"), true)
    }

    fn instances(&mut self, value: Option<&Value>, group: bool) -> Result<(), Failure> {
        let Some(value) = value.filter(|v| !v.is_null()) else {
            self.complete = false;
            return Ok(());
        };
        let rows = value
            .as_array()
            .filter(|rows| rows.len() <= 128)
            .ok_or_else(response)?;
        let mut seen = BTreeSet::new();
        let mut counts = Vec::new();
        for row in rows {
            let count = instance(row, group)?;
            let key = if group {
                "InstanceType"
            } else {
                "ResourceType"
            };
            let name = row[key].as_str().ok_or_else(response)?;
            if !seen.insert(name) {
                return Err(response());
            }
            self.complete &= count.name.is_some() && count.value.is_some();
            let fields: &[&str] = if group {
                &["InstanceType", "InstanceCount"]
            } else {
                &["ResourceType", "ResourceCount", "ResourceName"]
            };
            self.complete &= keys(row, fields)?;
            counts.push(count);
        }
        self.instances = Some(counts);
        Ok(())
    }
}

fn instance(row: &Value, group: bool) -> Result<Count, Failure> {
    let (kind, field) = if group {
        ("InstanceType", "InstanceCount")
    } else {
        ("ResourceType", "ResourceCount")
    };
    let name = row[kind].as_str().ok_or_else(response)?;
    if name.is_empty()
        || name.len() > 64
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(response());
    }
    let known = VPC.contains(&name) || GROUP.contains(&name);
    Ok(Count {
        name: known.then(|| name.into()),
        value: count(&row[field])?,
    })
}
