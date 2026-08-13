# Migrating to Runseal v0.17.0

Existing profiles remain compatible. A Cloudflare seat supplies an account and
either an API token file or API token:

```toml
[env.vars]
CLOUDFLARE_ACCOUNT_ID = "account-id"
CLOUDFLARE_API_TOKEN_FILE = "local://secrets/cloudflare"
```

Use `@cloudflare --help` for the complete atomic verb map. Token creation and
roll require `--value-file`; the destination must not already exist.

Rust consumers should keep `default-features = false`. Existing calls to
`runseal::tool::call` remain valid. Call `runseal::tool::cloudflare::invoke`
when supplying an in-memory policy body or receiving a token secret.

Code that constructed `tool::Reply` with a struct literal must switch to its
public constructors and accessors because the secret field is intentionally
private.
