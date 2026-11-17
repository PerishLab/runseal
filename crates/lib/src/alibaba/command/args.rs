use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "@alibaba",
    about = "One native Alibaba Cloud China-site authorization or identity operation",
    after_help = "Runseal implements the official CLI application's CN OAuth PKCE protocol over fixed HTTPS endpoints. No Alibaba CLI process or credential configuration is used. Browser login, MFA and consent remain human actions.\n\nBegin returns an authorization URL. Save the full redirected callback URL in a private file; never paste it into chat or pass the authorization code as an argument. Complete consumes --response-file after private custody. It does not acquire STS credentials or verify identity automatically.\n\nRefresh explicitly renews OAuth tokens. Renew separately exchanges OAuth access for STS credentials. Verify identity with the owning --account after renewal. Supply --principal to check a previously selected caller; omit it for initial owner-bound identity discovery. Recover only processes a retained private response without network replay. Unknown token-changing outcomes prevent automatic replay; inspect status before recovery or explicit reauthorization.\n\nPrivate state and callback files require trusted ancestors and owner-only access. Local logout does not revoke the remote application grant. OAuth bootstrap supports RAM users; Ensign federation uses separate custody and exact owner/issuer/subject/audience/provider/role bindings. Federate acquires an Ensign token, renew explicitly calls AssumeRoleWithOIDC, and role separately verifies GetCallerIdentity including stable RoleId and session ARN. Profile ENSIGN_HOME supplies the Ensign directory; ALIBABA_CLOUD_FEDERATION_FILE supplies a public binding JSON when flags are absent. No resource mutation or arbitrary endpoint override is exposed."
)]
pub(super) struct Input {
    #[arg(long, global = true)]
    pub state: Option<PathBuf>,
    #[arg(long, global = true)]
    pub account: Option<String>,
    #[arg(long, global = true)]
    pub principal: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub deed: Deed,
}

#[derive(Subcommand)]
pub(super) enum Deed {
    #[command(subcommand)]
    Auth(Auth),
    #[command(
        about = "Observe the non-root caller under an explicit owning account; optionally check a known principal"
    )]
    Identity,
    #[command(
        about = "Verify the exact retained federated role and session under the declared owner"
    )]
    Role,
}

#[derive(Subcommand)]
pub(super) enum Auth {
    #[command(
        about = "Acquire a resource token through the public Ensign SDK; does not dispatch STS"
    )]
    Federate {
        #[arg(long = "binding-file")]
        binding: Option<PathBuf>,
        #[arg(long = "ensign-home")]
        home: Option<PathBuf>,
    },
    #[command(
        about = "Persist PKCE state and return a browser authorization URL without opening a listener"
    )]
    Begin,
    #[command(
        about = "Exchange one private callback for OAuth tokens; does not acquire STS credentials"
    )]
    Complete {
        #[arg(long = "response-file")]
        response: PathBuf,
    },
    #[command(about = "Explicitly refresh OAuth tokens only")]
    Refresh,
    #[command(about = "Explicitly acquire STS credentials; verify identity separately afterward")]
    Renew,
    #[command(about = "Revalidate a retained private token response without any network call")]
    Recover,
    #[command(
        about = "Inspect finite local authorization and uncertainty facts without cloud calls"
    )]
    Status,
    #[command(about = "Clear local custody only; does not revoke remote authorization")]
    Logout,
}

#[cfg(test)]
mod tests {
    use super::Input;
    use clap::Parser;

    #[test]
    fn closed() {
        for words in [
            vec!["@alibaba", "auth", "complete", "--code", "WITHHELD_FIXTURE"],
            vec![
                "@alibaba",
                "--endpoint",
                "https://invalid.example",
                "identity",
            ],
            vec!["@alibaba", "--raw-body", "WITHHELD_FIXTURE", "identity"],
        ] {
            assert!(Input::try_parse_from(words).is_err());
        }
        assert!(Input::try_parse_from(["@alibaba", "auth", "complete"]).is_err());
    }
}
