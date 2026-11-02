use crate::tencent::tat;
use clap::{Args, Subcommand};

#[derive(Args)]
pub(in crate::tencent) struct Request {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Regions,
    Agent {
        #[arg(long)]
        id: String,
    },
}

impl From<&Request> for tat::Request {
    fn from(value: &Request) -> Self {
        Self {
            region: value.region.clone(),
            operation: match &value.operation {
                Operation::Regions => tat::Operation::Regions,
                Operation::Agent { id } => tat::Operation::Lighthouse { id: id.clone() },
            },
        }
    }
}
