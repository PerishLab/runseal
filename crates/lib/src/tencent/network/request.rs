use super::{Group, Operation, Page, Request, Subnet, Vpc, input, names::Names};
use crate::tencent::{Failure, api};
use serde_json::{Value, json};

pub(super) struct Call {
    pub action: &'static str,
    pub body: Value,
    pub kind: Kind,
}

pub(super) enum Kind {
    Vpc,
    Subnet,
    Group,
    Policies,
    Ack,
    List(Page, &'static str),
}

impl Request {
    pub(super) fn call(&self) -> Result<Call, Failure> {
        Names::region(&self.region)?;
        match &self.operation {
            Operation::Vpc(operation) => operation.call(),
            Operation::Subnet(operation) => operation.call(&self.region),
            Operation::Group(operation) => operation.call(),
        }
    }
}

impl Call {
    pub(super) fn request<'a>(&'a self, region: &'a str) -> api::Request<'a> {
        api::Request {
            service: "vpc",
            action: self.action,
            region: Some(region),
            params: &self.body,
        }
    }

    pub(super) fn mutation(&self) -> bool {
        !self.action.starts_with("Describe")
    }
}

impl Vpc {
    fn call(&self) -> Result<Call, Failure> {
        match self {
            Self::Get(id) => get("DescribeVpcs", "VpcIds", &id.id, "vpc-"),
            Self::List(page) => page.call("DescribeVpcs", "VpcSet"),
            Self::Create { name, cidr } => {
                Names::text(name, 60)?;
                Names::cidr(cidr, true)?;
                Ok(Call {
                    action: "CreateVpc",
                    body: json!({"VpcName":name,"CidrBlock":cidr}),
                    kind: Kind::Vpc,
                })
            }
            Self::Rename { id, name } => {
                Names::id(id, "vpc-")?;
                Names::text(name, 60)?;
                Ok(Call {
                    action: "ModifyVpcAttribute",
                    body: json!({"VpcId":id,"VpcName":name}),
                    kind: Kind::Ack,
                })
            }
            Self::Delete(id) => delete("DeleteVpc", "VpcId", &id.id, "vpc-"),
        }
    }
}

impl Subnet {
    fn call(&self, region: &str) -> Result<Call, Failure> {
        match self {
            Self::Get(id) => get("DescribeSubnets", "SubnetIds", &id.id, "subnet-"),
            Self::List(page) => page.call("DescribeSubnets", "SubnetSet"),
            Self::Create {
                name,
                vpc,
                zone,
                cidr,
            } => {
                Names::text(name, 60)?;
                Names::id(vpc, "vpc-")?;
                Names::cidr(cidr, true)?;
                let suffix = zone.strip_prefix(&format!("{region}-")).ok_or_else(input)?;
                if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(input());
                }
                Ok(Call {
                    action: "CreateSubnet",
                    body: json!({"SubnetName":name,"VpcId":vpc,"Zone":zone,"CidrBlock":cidr}),
                    kind: Kind::Subnet,
                })
            }
            Self::Rename { id, name } => {
                Names::id(id, "subnet-")?;
                Names::text(name, 60)?;
                Ok(Call {
                    action: "ModifySubnetAttribute",
                    body: json!({"SubnetId":id,"SubnetName":name}),
                    kind: Kind::Ack,
                })
            }
            Self::Delete(id) => delete("DeleteSubnet", "SubnetId", &id.id, "subnet-"),
        }
    }
}

impl Group {
    fn call(&self) -> Result<Call, Failure> {
        match self {
            Self::Get(id) => get("DescribeSecurityGroups", "SecurityGroupIds", &id.id, "sg-"),
            Self::List(page) => page.call("DescribeSecurityGroups", "SecurityGroupSet"),
            Self::Create {
                name,
                description,
                project,
            } => {
                Names::text(name, 60)?;
                Names::text(description, 100)?;
                Ok(Call {
                    action: "CreateSecurityGroup",
                    body: json!({"GroupName":name,"GroupDescription":description,"ProjectId":project.to_string()}),
                    kind: Kind::Group,
                })
            }
            Self::Modify {
                id,
                name,
                description,
            } => {
                Names::id(id, "sg-")?;
                Names::text(name, 60)?;
                Names::text(description, 100)?;
                Ok(Call {
                    action: "ModifySecurityGroupAttribute",
                    body: json!({"SecurityGroupId":id,"GroupName":name,"GroupDescription":description}),
                    kind: Kind::Ack,
                })
            }
            Self::Delete(id) => delete("DeleteSecurityGroup", "SecurityGroupId", &id.id, "sg-"),
            Self::Rules(id) => {
                Names::id(&id.id, "sg-")?;
                Ok(Call {
                    action: "DescribeSecurityGroupPolicies",
                    body: json!({"SecurityGroupId":id.id}),
                    kind: Kind::Policies,
                })
            }
            Self::Reset { id, rules } => {
                Names::id(id, "sg-")?;
                Ok(Call {
                    action: "ModifySecurityGroupPolicies",
                    body: json!({"SecurityGroupId":id,"SecurityGroupPolicySet":rules.body()?,"SortPolicys":true}),
                    kind: Kind::Ack,
                })
            }
        }
    }
}

impl Page {
    fn call(self, action: &'static str, field: &'static str) -> Result<Call, Failure> {
        if !(1..=100).contains(&self.limit) {
            return Err(input());
        }
        Ok(Call {
            action,
            body: json!({"Offset":self.offset.to_string(),"Limit":self.limit.to_string()}),
            kind: Kind::List(self, field),
        })
    }
}

fn get(action: &'static str, field: &str, id: &str, prefix: &str) -> Result<Call, Failure> {
    Names::id(id, prefix)?;
    Ok(Call {
        action,
        body: json!({field:[id],"Offset":"0","Limit":"1"}),
        kind: Kind::List(
            Page {
                offset: 0,
                limit: 1,
            },
            match prefix {
                "vpc-" => "VpcSet",
                "subnet-" => "SubnetSet",
                _ => "SecurityGroupSet",
            },
        ),
    })
}

fn delete(action: &'static str, field: &str, id: &str, prefix: &str) -> Result<Call, Failure> {
    Names::id(id, prefix)?;
    Ok(Call {
        action,
        body: json!({field:id}),
        kind: Kind::Ack,
    })
}
