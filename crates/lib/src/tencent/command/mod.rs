mod auth;
pub(super) mod cam;
mod compute;
pub(super) mod dns;
pub(super) use dns::Dns;
mod request;
pub(super) use request::{Mode, Request};

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "@tencent",
    about = "One native Tencent account, CAM, DNSPod, network or compute operation",
    after_help = "Profile references: TENCENTCLOUD_STATE_FILE selects an absolute private state file; TENCENTCLOUD_ACCOUNT_UIN declares the expected owning account. Profiles remain env, argv and symlink only.\n\nAuthorize with auth begin, complete the official Tencent Cloud CLI application's browser login, save the full return in a private file, then auth complete --response-file PATH. The return contains bearer tokens: never pass it as an argument or paste it into chat. Complete consumes that file after saving authorization. Verify identity before mutations and declare --account.\n\nRefresh renews the OAuth access token; renew separately acquires cloud credentials and clears the identity binding. Neither runs implicitly. Logout clears local custody only, without remote revocation. Key creation requires an explicit delegated user and reserves a new private --value-file before contacting CAM.\n\nAll private files require trusted ancestors and owner-only access (Unix 0600 files/0700 directories; Windows protected ACLs). Create the parent first. Responses omit credentials and raw provider error messages. An uncertain remote result is never retried automatically: inspect the exact resource before recovery. DNS values are ordinary command arguments; do not use this surface to publish secret-bearing record values. Mainland Tencent Cloud only."
)]
pub(super) struct Input {
    #[arg(long, global = true)]
    pub state: Option<PathBuf>,
    #[arg(long, global = true)]
    pub account: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub deed: Deed,
}

#[derive(Subcommand)]
pub(super) enum Deed {
    #[command(subcommand)]
    Auth(Auth),
    Identity,
    Network(super::network::Request),
    #[command(subcommand)]
    Compute(compute::Query),
    #[command(subcommand)]
    Cam(Cam),
    #[command(subcommand)]
    Dns(Dns),
}

#[derive(Subcommand)]
pub(super) enum Auth {
    #[command(about = "Begin a bounded browser authorization without opening a listener")]
    Begin,
    #[command(about = "Consume a private browser return and acquire temporary cloud credentials")]
    Complete {
        #[arg(long = "response-file")]
        response: PathBuf,
    },
    #[command(about = "Explicitly refresh the OAuth access token only")]
    Refresh,
    #[command(about = "Explicitly acquire cloud credentials; verify identity again afterward")]
    Renew,
    Status,
    #[command(about = "Clear local authorization; does not revoke the remote grant")]
    Logout,
}

#[derive(Subcommand)]
pub(super) enum Cam {
    #[command(subcommand)]
    User(User),
    #[command(subcommand)]
    Policy(Policy),
    #[command(subcommand)]
    Key(Key),
}

#[derive(Subcommand)]
pub(super) enum User {
    Get(Name),
    Create(Name),
}

#[derive(Args)]
pub(super) struct Name {
    #[arg(long)]
    pub name: String,
}

#[derive(Subcommand)]
pub(super) enum Policy {
    List {
        #[arg(long, default_value = "1")]
        page: u32,
    },
    Get {
        #[arg(long)]
        id: u64,
    },
    Create {
        #[arg(long)]
        name: String,
        #[arg(long = "document-file")]
        document: PathBuf,
    },
    Attach(Binding),
    Detach(Binding),
}

#[derive(Args)]
pub(super) struct Binding {
    #[arg(long)]
    pub id: u64,
    #[arg(long)]
    pub user: u64,
}

#[derive(Subcommand)]
pub(super) enum Key {
    List {
        #[arg(long)]
        user: u64,
    },
    Create {
        #[arg(long)]
        user: u64,
        #[arg(long = "value-file")]
        output: PathBuf,
    },
    Disable(Handle),
    Delete(Handle),
}

#[derive(Args)]
pub(super) struct Handle {
    #[arg(long)]
    pub user: u64,
    #[arg(long)]
    pub id: String,
}

pub(super) fn receipt(valid: bool) -> anyhow::Result<()> {
    if !valid {
        anyhow::bail!(
            "Tencent operation receipt incomplete; a mutation may have completed; inspect the exact resource before recovery"
        );
    }
    Ok(())
}

#[cfg(test)]
mod checks;
