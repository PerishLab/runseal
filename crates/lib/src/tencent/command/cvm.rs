use super::super::cvm as sdk;
use clap::{Args, Subcommand, ValueEnum};
use std::{io::Read, path::PathBuf};

#[derive(Args)]
#[command(
    about = "One scoped CVM operation; one hourly instance, no waiter or retry",
    after_help = "Quote/quota/DryRun are not purchase or readiness guarantees. Run requires an explicit client token; persist it with the exact input before sending. Accepted writes are not convergence. Exact empty results are not an absence guarantee. Inspect foreign disks/EIPs/keys before termination: no attachment cleanup or compensation is performed. Key import accepts only OpenSSH RSA public material; private keys never belong here."
)]
pub(in crate::tencent) struct Request {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Quote(Shape),
    Quota {
        #[arg(long)]
        zone: Option<String>,
    },
    Run {
        #[command(flatten)]
        shape: Shape,
        #[arg(long = "client-token")]
        token: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Get(Id),
    Status(Id),
    Rename {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    Terminate(Id),
    #[command(subcommand)]
    Key(Key),
}

#[derive(Args)]
struct Id {
    #[arg(long)]
    id: String,
}

#[derive(Subcommand)]
enum Key {
    Import {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 0)]
        project: u64,
        #[arg(long = "public-file")]
        public: PathBuf,
    },
    Get(Id),
    Delete(Id),
}

#[derive(Args)]
struct Shape {
    #[arg(long)]
    name: String,
    #[arg(long)]
    zone: String,
    #[arg(long, default_value_t = 0)]
    project: u64,
    #[arg(long)]
    vpc: String,
    #[arg(long)]
    subnet: String,
    #[arg(long)]
    flavor: String,
    #[arg(long)]
    image: String,
    #[arg(long = "disk-class", value_enum)]
    storage: Storage,
    #[arg(long = "disk-size")]
    size: u32,
    #[arg(long = "group", required = true)]
    groups: Vec<String>,
    #[arg(long)]
    key: String,
    #[arg(long)]
    public: bool,
    #[arg(long, default_value_t = 0)]
    bandwidth: u32,
}

#[derive(Clone, Copy, ValueEnum)]
enum Storage {
    #[value(name = "CLOUD_PREMIUM")]
    Premium,
    #[value(name = "CLOUD_SSD")]
    Ssd,
}

impl Shape {
    fn spec(&self) -> sdk::Spec {
        sdk::Spec {
            name: self.name.clone(),
            placement: sdk::Placement {
                zone: self.zone.clone(),
                project: self.project,
                vpc: self.vpc.clone(),
                subnet: self.subnet.clone(),
            },
            hardware: sdk::Hardware {
                flavor: self.flavor.clone(),
                image: self.image.clone(),
                disk: sdk::Disk {
                    size: self.size,
                    class: match self.storage {
                        Storage::Premium => sdk::Storage::Premium,
                        Storage::Ssd => sdk::Storage::Ssd,
                    },
                },
            },
            connection: sdk::Connection {
                groups: self.groups.clone(),
                key: self.key.clone(),
                public: self.public,
                bandwidth: self.bandwidth,
            },
        }
    }
}

pub(in crate::tencent) fn request(request: &Request) -> anyhow::Result<sdk::Request> {
    let operation = match &request.operation {
        Operation::Quote(shape) => sdk::Operation::Quote(shape.spec()),
        Operation::Quota { zone } => sdk::Operation::Quota { zone: zone.clone() },
        Operation::Run { shape, token, dry } => sdk::Operation::Run {
            spec: shape.spec(),
            token: token.clone(),
            dry: *dry,
        },
        Operation::Get(id) => sdk::Operation::Get { id: id.id.clone() },
        Operation::Status(id) => sdk::Operation::Status { id: id.id.clone() },
        Operation::Rename { id, name } => sdk::Operation::Rename {
            id: id.clone(),
            name: name.clone(),
        },
        Operation::Terminate(id) => sdk::Operation::Terminate { id: id.id.clone() },
        Operation::Key(Key::Get(id)) => sdk::Operation::Key { id: id.id.clone() },
        Operation::Key(Key::Delete(id)) => sdk::Operation::Delete { id: id.id.clone() },
        Operation::Key(Key::Import {
            name,
            project,
            public,
        }) => {
            let mut text = zeroize::Zeroizing::new(String::new());
            std::fs::File::open(public)
                .and_then(|file| file.take(8193).read_to_string(&mut text))
                .map_err(|_| sdk::input())?;
            if text.len() > 8192 {
                return Err(sdk::input().into());
            }
            let key = sdk::Import {
                name: name.clone(),
                project: *project,
                public: text.trim_end_matches(['\r', '\n']).into(),
            };
            key.validate()?;
            sdk::Operation::Import(key)
        }
    };
    let request = sdk::Request {
        region: request.region.clone(),
        operation,
    };
    request.validate()?;
    Ok(request)
}
