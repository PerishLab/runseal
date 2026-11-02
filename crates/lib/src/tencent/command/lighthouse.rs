use super::super::lighthouse as sdk;
use clap::{Args, Subcommand};

#[derive(Args)]
#[command(
    about = "One Lighthouse instance/key observation; no enrollment or lifecycle mutation",
    after_help = "Preserve nullable facts and schema coverage. Empty key associations and LoginSettings.KeyIds are not proof of absent guest access. Exact unlisted is not absence. Per-user associations and aggregate IDs are independent observations. Latest-operation SUCCESS must match the caller's exact operation; no polling, refresh, retry, trust registration or key mutation follows."
)]
pub(in crate::tencent) struct Request {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Instance(Id),
    Key(Id),
    Keys {
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Args)]
struct Id {
    #[arg(long)]
    id: String,
}

impl Request {
    pub(in crate::tencent) fn convert(&self) -> Result<sdk::Request, crate::tencent::Failure> {
        let operation = match &self.operation {
            Operation::Instance(id) => sdk::Operation::Instance { id: id.id.clone() },
            Operation::Key(id) => sdk::Operation::Key { id: id.id.clone() },
            Operation::Keys { offset, limit } => sdk::Operation::Keys(sdk::Window {
                offset: *offset,
                limit: *limit,
            }),
        };
        let request = sdk::Request {
            region: self.region.clone(),
            operation,
        };
        request.validate()?;
        Ok(request)
    }
}
