use super::super::lighthouse as sdk;
use clap::{Args, Subcommand};
use std::{io::Read, path::PathBuf};

#[derive(Args)]
#[command(
    about = "One Lighthouse observation or public-key operation; no enrollment workflow",
    after_help = "Preserve nullable facts and schema coverage. Empty key associations and LoginSettings.KeyIds are not proof of absent guest access. Exact unlisted is not absence. Per-user associations and aggregate IDs are independent observations. Latest-operation SUCCESS must match the caller's exact operation; Persist exact intent before writes: no client token, polling, refresh, retry or trust registration. ONLINE is explicit and never falls back to OFFLINE; existing passwords or keys may become invalid. Accepted is not binding convergence or SSH authentication."
)]
pub(in crate::tencent) struct Request {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Import {
        #[arg(long)]
        name: String,
        #[arg(long = "public-file")]
        public: PathBuf,
    },
    Delete(Id),
    Associate {
        #[arg(long)]
        key: String,
        #[arg(long)]
        instance: String,
        #[arg(long)]
        username: String,
    },
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
            Operation::Import { name, public } => {
                let mut text = zeroize::Zeroizing::new(String::new());
                std::fs::File::open(public)
                    .and_then(|file| file.take(8193).read_to_string(&mut text))
                    .map_err(|_| sdk::input())?;
                if text.len() > 8192 {
                    return Err(sdk::input());
                }
                sdk::Operation::Import(sdk::Import {
                    name: name.clone(),
                    public: text.trim_end_matches(['\r', '\n']).into(),
                })
            }
            Operation::Delete(id) => sdk::Operation::Delete { id: id.id.clone() },
            Operation::Associate {
                key,
                instance,
                username,
            } => sdk::Operation::Associate(sdk::Binding {
                key: key.clone(),
                instance: instance.clone(),
                username: username.clone(),
            }),
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
