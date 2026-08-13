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
