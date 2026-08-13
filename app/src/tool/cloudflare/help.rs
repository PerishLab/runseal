pub(super) const TEXT: &str = r#"@cloudflare — one Cloudflare HTTP control-plane operation

Usage: runseal :PROFILE @cloudflare [OPTIONS] token OWNER VERB [ID]

Owners:
  account   Account-owned API tokens; requires --account or CLOUDFLARE_ACCOUNT_ID
  user      User-owned API tokens

Verbs:
  token OWNER list
  token OWNER show ID
  token OWNER create --body-file PATH --value-file PATH
  token OWNER edit ID --body-file PATH
  token OWNER roll ID --value-file PATH
  token OWNER delete ID
  token OWNER verify
  token OWNER permission list [--name NAME] [--scope SCOPE]
  worker service show NAME
  worker domain list
  r2 bucket show NAME
  r2 bucket domain list NAME
  r2 bucket domain delete NAME DOMAIN
  r2 bucket delete NAME

Options:
  --json
  --url URL                 Default: https://api.cloudflare.com/client/v4
  --account ID
  --token-file PATH
  --body-file PATH
  --value-file PATH         Must not already exist; created mode 0600
  --direction asc|desc
  --include-expired true|false
  --limit COUNT

Profile variables:
  CLOUDFLARE_API_URL
  CLOUDFLARE_ACCOUNT_ID
  CLOUDFLARE_API_TOKEN_FILE or CLOUDFLARE_API_TOKEN
  CLOUDFLARE_API_KEY_FILE or CLOUDFLARE_API_KEY, with CLOUDFLARE_API_EMAIL
"#;
