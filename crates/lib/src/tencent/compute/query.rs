use super::super::{Class, Failure, api};
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Product {
    Cvm,
    Lighthouse,
}

impl Product {
    pub(super) fn service(self) -> &'static str {
        match self {
            Self::Cvm => "cvm",
            Self::Lighthouse => "lighthouse",
        }
    }
}

#[derive(Clone, Copy, Debug, Args)]
pub struct Window {
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Query {
    Regions {
        #[arg(long)]
        product: Product,
    },
    Zones {
        #[arg(long)]
        product: Product,
        #[arg(long)]
        region: String,
    },
    Instances {
        #[arg(long)]
        product: Product,
        #[arg(long)]
        region: String,
        #[command(flatten)]
        window: Window,
    },
    Shapes {
        #[arg(long)]
        region: String,
        #[arg(long)]
        zone: Option<String>,
    },
    Images {
        #[arg(long)]
        region: String,
        #[command(flatten)]
        window: Window,
    },
    Blueprints {
        #[arg(long)]
        region: String,
        #[command(flatten)]
        window: Window,
    },
    Bundles {
        #[arg(long)]
        region: String,
        #[arg(long)]
        blueprint: Option<String>,
        #[command(flatten)]
        window: Window,
    },
}

pub(super) enum Kind {
    Regions,
    Zones,
    Instances(Product, Window),
    Shapes,
    Images(Window),
    Blueprints(Window),
    Bundles(Window),
}

pub(in crate::tencent) struct Call {
    pub product: Product,
    pub service: &'static str,
    pub action: &'static str,
    pub region: Option<String>,
    pub body: Value,
    pub(super) kind: Kind,
}

impl Query {
    pub(in crate::tencent) fn call(&self) -> Result<Call, Failure> {
        let call = match self {
            Self::Regions { product } => Call {
                product: *product,
                service: "region",
                action: "DescribeRegions",
                region: None,
                body: json!({"Product": product.service()}),
                kind: Kind::Regions,
            },
            Self::Zones { product, region } => Call {
                product: *product,
                service: product.service(),
                action: "DescribeZones",
                region: Some(region.clone()),
                body: json!({}),
                kind: Kind::Zones,
            },
            Self::Instances {
                product,
                region,
                window,
            } => Call {
                product: *product,
                service: product.service(),
                action: "DescribeInstances",
                region: Some(region.clone()),
                body: window.body()?,
                kind: Kind::Instances(*product, *window),
            },
            Self::Shapes { region, zone } => Call {
                product: Product::Cvm,
                service: "cvm",
                action: "DescribeZoneInstanceConfigInfos",
                region: Some(region.clone()),
                body: shape(region, zone.as_deref())?,
                kind: Kind::Shapes,
            },
            Self::Images { region, window } => Call {
                product: Product::Cvm,
                service: "cvm",
                action: "DescribeImages",
                region: Some(region.clone()),
                body: window.filtered(json!([{"Name":"image-type","Values":["PUBLIC_IMAGE"]}]))?,
                kind: Kind::Images(*window),
            },
            Self::Blueprints { region, window } => Call {
                product: Product::Lighthouse,
                service: "lighthouse",
                action: "DescribeBlueprints",
                region: Some(region.clone()),
                body: window.filtered(json!([{"Name":"blueprint-type","Values":["PURE_OS"]}]))?,
                kind: Kind::Blueprints(*window),
            },
            Self::Bundles {
                region,
                blueprint,
                window,
            } => Call {
                product: Product::Lighthouse,
                service: "lighthouse",
                action: "DescribeBundles",
                region: Some(region.clone()),
                body: bundle(*window, blueprint.as_deref())?,
                kind: Kind::Bundles(*window),
            },
        };
        if let Some(region) = &call.region {
            location(region)?;
        }
        Ok(call)
    }
}

fn bundle(window: Window, blueprint: Option<&str>) -> Result<Value, Failure> {
    let mut body = window.body()?;
    if let Some(blueprint) = blueprint {
        token(blueprint)?;
        if !blueprint.starts_with("lhbp-") || blueprint.len() <= 5 {
            return Err(input());
        }
        body["BlueprintId"] = json!(blueprint);
    }
    Ok(body)
}

impl Call {
    pub(in crate::tencent) fn request(&self) -> api::Request<'_> {
        api::Request {
            service: self.service,
            action: self.action,
            region: self.region.as_deref(),
            params: &self.body,
        }
    }
}

impl Window {
    fn body(self) -> Result<Value, Failure> {
        if !(1..=100).contains(&self.limit) {
            return Err(input());
        }
        Ok(json!({"Offset": self.offset, "Limit": self.limit}))
    }

    fn filtered(self, filters: Value) -> Result<Value, Failure> {
        let mut body = self.body()?;
        body["Filters"] = filters;
        Ok(body)
    }
}

fn shape(region: &str, zone: Option<&str>) -> Result<Value, Failure> {
    if let Some(zone) = zone {
        token(zone)?;
        if !zone.starts_with(&format!("{region}-")) {
            return Err(input());
        }
        Ok(json!({"Filters":[{"Name":"zone","Values":[zone]}]}))
    } else {
        Ok(json!({}))
    }
}

pub(super) fn token(value: &str) -> Result<(), Failure> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    {
        return Err(input());
    }
    Ok(())
}

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "Tencent compute query refused; use explicit supported product, region, identifiers and a page limit from 1 to 100",
    )
}

fn location(value: &str) -> Result<(), Failure> {
    let lexical = value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !lexical || value.len() > 64 {
        return Err(input());
    }
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return Err(input());
    }
    Ok(())
}
