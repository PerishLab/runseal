use super::Rules;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct Request {
    #[arg(long)]
    pub region: String,
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Operation {
    #[command(subcommand)]
    Vpc(Vpc),
    #[command(subcommand)]
    Subnet(Subnet),
    #[command(subcommand)]
    Group(Group),
}

#[derive(Clone, Debug, Subcommand)]
pub enum Vpc {
    Get(Id),
    Survey(Id),
    List(Page),
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        cidr: String,
    },
    Rename {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    Delete(Id),
}

#[derive(Clone, Debug, Subcommand)]
pub enum Subnet {
    Get(Id),
    Survey(Id),
    List(Page),
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        vpc: String,
        #[arg(long)]
        zone: String,
        #[arg(long)]
        cidr: String,
    },
    Rename {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
    },
    Delete(Id),
}

#[derive(Clone, Debug, Subcommand)]
pub enum Group {
    Get(Id),
    Associations(Id),
    List(Page),
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        project: u64,
    },
    Modify {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
    },
    Delete(Id),
    Rules(Id),
    Reset {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        rules: Rules,
    },
}

#[derive(Clone, Debug, Args)]
pub struct Id {
    #[arg(long)]
    pub id: String,
}

#[derive(Clone, Copy, Debug, Args)]
pub struct Page {
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}
