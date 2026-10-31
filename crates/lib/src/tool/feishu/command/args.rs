use super::super::Actor;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "@feishu",
    about = "Native Feishu operations; custom-app bot/user authorization and atomic text messages. No automatic login, renewal, retries, enrichment or pagination."
)]
pub(crate) struct Input {
    #[arg(
        long,
        global = true,
        help = "Absolute private state file; otherwise profile FEISHU_STATE_FILE"
    )]
    pub(super) state: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Expected App ID; otherwise profile FEISHU_APP_ID"
    )]
    pub(super) app: Option<String>,
    #[arg(
        long = "as",
        global = true,
        value_enum,
        help = "Explicit actor; otherwise profile FEISHU_ACTOR, then bot"
    )]
    pub(super) actor: Option<Actor>,
    #[arg(
        long = "user",
        global = true,
        help = "Expected user open_id; otherwise profile FEISHU_USER_OPEN_ID; required for --as user"
    )]
    pub(super) user: Option<String>,
    #[arg(
        long,
        global = true,
        help = "Structured output; failures also return nonzero"
    )]
    pub json: bool,
    #[command(subcommand)]
    pub(super) deed: Deed,
}

#[derive(Subcommand)]
pub(super) enum Deed {
    #[command(
        about = "Explicit app custody, user authorization, actor-specific renewal and identity verification"
    )]
    Auth {
        #[command(subcommand)]
        deed: Auth,
    },
    #[command(about = "Instant messaging; no hidden name/reaction/thread/resource enrichment")]
    Im {
        #[command(subcommand)]
        deed: Im,
    },
}

#[derive(Subcommand)]
pub(super) enum Auth {
    #[command(
        about = "Import private JSON {app_id,app_secret} into a new private state; no network call"
    )]
    Import {
        #[arg(long = "credentials-file")]
        credentials: PathBuf,
    },
    #[command(
        about = "Read nonsecret local state, selected actor and retained-return status; no network or implicit refresh",
        long_about = "Read nonsecret local state without network access or implicit refresh. For a retained token return, response.token/refresh are byte lengths, bearer is a type check, lifetime/lease are observed duration metadata and scopes is a count. No token values or raw provider prose are returned. If retained=true, auth recover can revalidate the same return locally after compatibility repair; never repeat an uncertain exchange/renew."
    )]
    Status,
    #[command(
        about = "Begin one explicit user device authorization request; browser consent remains human; scopes are exactly caller supplied, include offline_access for refresh"
    )]
    Begin {
        #[arg(long = "scope", required = true)]
        scopes: Vec<String>,
    },
    #[command(
        about = "Exchange the pending device code once, respecting saved polling interval; no automatic polling or retries"
    )]
    Exchange,
    #[command(
        about = "Recover a retained token return locally; no network request or replay. Inspect auth status retained first; unavailable or invalid returns remain refused"
    )]
    Recover,
    #[command(about = "Discard only local user authorization; no remote revocation promise")]
    Logout,
    #[command(
        about = "Explicit bot client_credentials issuance or user refresh_token renewal; one request, clears the selected actor identity binding; no retry"
    )]
    Renew,
    #[command(
        about = "Verify selected actor with bot info or user_info; one request binds app/current token and expected user for user mode; does not prove resource permission"
    )]
    Identity,
}

#[derive(Subcommand)]
pub(super) enum Im {
    Messages {
        #[command(subcommand)]
        deed: Messages,
    },
}

#[derive(Subcommand)]
pub(super) enum Messages {
    #[command(
        about = "Send one plain-text message; explicit destination and UUID. Accepted is not delivery; caller owns durable intent and unknown-write recovery"
    )]
    Send {
        #[arg(
            long = "chat-id",
            conflicts_with = "recipient",
            required_unless_present = "recipient"
        )]
        chat: Option<String>,
        #[arg(
            long = "user-id",
            conflicts_with = "chat",
            required_unless_present = "chat"
        )]
        recipient: Option<String>,
        #[arg(long = "text-file")]
        text: PathBuf,
        #[arg(long = "idempotency-key")]
        uuid: String,
    },
    #[command(
        about = "Read one page from one exact chat; no enrichment, automatic pagination or peer resolution"
    )]
    List {
        #[arg(long = "chat-id")]
        chat: String,
        #[arg(long = "page-size", default_value = "50")]
        size: u8,
        #[arg(long = "page-token")]
        cursor: Option<String>,
        #[arg(long = "start-time")]
        start: Option<i64>,
        #[arg(long = "end-time")]
        end: Option<i64>,
        #[arg(long)]
        ascending: bool,
    },
    #[command(about = "Read one exact message, with sender IDs as open_id; no enrichment")]
    Get {
        #[arg(long = "message-id")]
        message: String,
    },
    #[command(
        about = "Reply once with plain text; caller owns sender policy and durable intent. Accepted does not prove delivery. UUID dedup is time-bounded (one hour), not durable exactly-once."
    )]
    Reply {
        #[arg(long = "message-id")]
        message: String,
        #[arg(
            long = "text-file",
            help = "Absolute private UTF-8 file, 1..20000 bytes"
        )]
        text: PathBuf,
        #[arg(
            long = "idempotency-key",
            help = "Explicit UUID, 1..50 visible ASCII bytes; persist exact target/content/UUID before dispatch"
        )]
        uuid: String,
        #[arg(long = "reply-in-thread")]
        thread: bool,
    },
}
