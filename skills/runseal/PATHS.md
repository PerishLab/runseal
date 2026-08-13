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
runseal :perish @forgejo status show <ref>
runseal :perish @forgejo workflow dispatch release.yml --ref main
runseal :perish @forgejo get https://releases.runseal.perish.uk/stable
```

`@forgejo` verbs: user show; issue show/list/create/edit; issue comment
list/create; pull show/list/create/edit/merge; review list/create; status
show; branch show/create; protection show/create/edit; repo show/edit/delete;
secret list/set/delete; workflow dispatch; run show; job log; label list;
get URL. One HTTP operation per invocation. Merge sends Forgejo `Do` and
`head_commit_id`. `get` is unauthenticated.

`@forgejo` reads `FORGEJO_URL` and either `FORGEJO_TOKEN_FILE` or
`FORGEJO_TOKEN` from the resolved profile. `--url` and `--token-file`
override. There is no login verb. Dialect is Forgejo 15.0.6.

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
