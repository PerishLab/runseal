use super::super::compute as sdk;
use clap::{Args, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(in crate::tencent) enum Product {
    Cvm,
    Lighthouse,
}

#[derive(Clone, Copy, Debug, Args)]
pub(in crate::tencent) struct Window {
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Clone, Debug, Subcommand)]
pub(in crate::tencent) enum Query {
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

impl From<Query> for sdk::Query {
    fn from(query: Query) -> Self {
        match query {
            Query::Regions { product } => Self::Regions {
                product: product.into(),
            },
            Query::Zones { product, region } => Self::Zones {
                product: product.into(),
                region,
            },
            Query::Instances {
                product,
                region,
                window,
            } => Self::Instances {
                product: product.into(),
                region,
                window: window.into(),
            },
            Query::Shapes { region, zone } => Self::Shapes { region, zone },
            Query::Images { region, window } => Self::Images {
                region,
                window: window.into(),
            },
            Query::Blueprints { region, window } => Self::Blueprints {
                region,
                window: window.into(),
            },
            Query::Bundles {
                region,
                blueprint,
                window,
            } => Self::Bundles {
                region,
                blueprint,
                window: window.into(),
            },
        }
    }
}

impl From<Product> for sdk::Product {
    fn from(product: Product) -> Self {
        match product {
            Product::Cvm => Self::Cvm,
            Product::Lighthouse => Self::Lighthouse,
        }
    }
}

impl From<Window> for sdk::Window {
    fn from(window: Window) -> Self {
        Self {
            offset: window.offset,
            limit: window.limit,
        }
    }
}
