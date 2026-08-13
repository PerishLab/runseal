# Migrating to Runseal v0.16.0

Create the operator Forgejo seat in the directory-form home profile:

```toml
# $RUNSEAL_HOME/profiles/perish/runseal.toml
[env.vars]
FORGEJO_URL = "https://git.perish.top"
FORGEJO_TOKEN_FILE = "local://secrets/forgejo"
```

Place only the token value in
`$RUNSEAL_HOME/profiles/perish/.local/secrets/forgejo`. Runseal does not import
or rewrite `tea.yml` and does not create a second credential store.

Inspect the available atomic operations and prove the seat with a read:

```bash
runseal :perish @forgejo --help
runseal :perish @forgejo user show
```

Rust consumers use the published registry package and the structured entry:

```toml
runseal = { version = "0.16.0", registry = "perish" }
```

Call `runseal::tool::call("forgejo", argv, vars)` and consume its `Reply`.
Supply `FORGEJO_URL` plus `FORGEJO_TOKEN_FILE` or `FORGEJO_TOKEN` in `vars`;
do not capture CLI stdout or reconstruct Forgejo routes in the consumer.

Existing external-command profiles and profile grammar require no migration.
