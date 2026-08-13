# Hot paths

## Inspect a profile

```bash
runseal profile
runseal profile NAME
```

The report names the home, profile, root, path, and the counts of `env`,
`argv`, and `symlink`. It does not print secret values.

## Run inside a profile

```bash
runseal : <command> [args...]
runseal :<name> <command> [args...]
```

Profile mode requires one external command or one `@tool`. Use it only when
that command needs the selected profile.

## Resolve conventional paths

```bash
runseal resolve resource://file
runseal resolve --profile NAME local://secrets/token
```

Each argument must be a `resource://` or `local://` URI. The printed paths are
traversal-resistant seats under `.runseal/resources` or `.local`.

## Home profiles

A named seat that is not in the repository walk-up is read from
`$RUNSEAL_HOME/profiles`. Two writings are valid; a flat file wins when both
exist:

```
$RUNSEAL_HOME/profiles/{name}.toml
$RUNSEAL_HOME/profiles/{name}/runseal.toml
```

The directory writing keeps `local://` under that seat:
`$RUNSEAL_HOME/profiles/{name}/.local`. Operator Forgejo seats use the
directory form:

```toml
[env.vars]
FORGEJO_URL = "https://git.perish.top"
FORGEJO_TOKEN_FILE = "local://secrets/forgejo"
```

```bash
runseal :perish @forgejo --json issue show 154
runseal :perish @forgejo pull merge 71 --head SHA
runseal :perish @forgejo task list <run-number>
runseal :perish @forgejo job log <run-number> <job-index> --watch
runseal :perish @forgejo --help
```

`@forgejo --help` is the complete verb map. Lists page fully; `--watch` belongs
only to `job log RUN JOB`. Merge sends Forgejo `Do` and `head_commit_id`.

`@forgejo` reads `FORGEJO_URL` and either `FORGEJO_TOKEN_FILE` or
`FORGEJO_TOKEN` from the resolved profile. `--url` and `--token-file`
override. There is no login verb. Dialect is Forgejo 15.0.6.

Cloudflare seats provide `CLOUDFLARE_ACCOUNT_ID` plus
`CLOUDFLARE_API_TOKEN_FILE` or `CLOUDFLARE_API_TOKEN`:

```bash
runseal :perish @cloudflare token account permission list
runseal :perish @cloudflare token user create --body-file policy.json --value-file token
runseal :perish @cloudflare worker domain list
runseal :perish @cloudflare r2 bucket show BUCKET
runseal :perish @cloudflare --help
```

Token owners are explicit. Create and roll reserve a new mode-0600 value file;
the secret never enters ordinary output. The tool also covers token lifecycle,
Worker perception, and R2 bucket/custom-domain teardown.

## Embed a native tool

Rust consumers call the same operation surface without CLI rendering:

```toml
runseal = { version = "0", registry = "perish", default-features = false }
```

Call `runseal::tool::call` for ordinary verbs. Cloudflare callers with an
in-memory policy use `tool::cloudflare::invoke`; a returned token value is a
redacted, zeroizing `Reply::secret`. Keep `default-features = false` in Plumb.

## Write a profile

```toml
[env]
unset = ["AMBIENT_KEY"]

[env.vars]
TOKEN_FILE = "local://secrets/token"
TEMPLATE = "resource://template.txt"

[argv]
cargo = ["--locked"]

[[symlink]]
source = "resource://config"
target = "local://run/config"
```

Committed inert material lives under `.runseal/resources`. Secrets and local
state live under `.local`. Neither seat carries executable behavior.

## Change the repository

Read the repository's own instructions. Work on a dedicated task branch. Run
the repository's complete validation before landing.

## Operate the skill seat

```bash
runseal skill status
runseal skill upgrade --dry-run
runseal skill upgrade
runseal skill stage --channel CHANNEL --version VERSION --path /isolated/runseal
runseal skill list
runseal skill uninstall
```

Managed operations accept stable. Stage accepts one exact non-stable version,
requires a new path ending in `runseal`, and never enters the managed ledger.
