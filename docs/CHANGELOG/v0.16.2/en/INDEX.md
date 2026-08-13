# Runseal v0.16.2

This is the first stable release of the 0.16 line. It delivers Runseal's first
native atomic tool and makes the same Forgejo operation surface available to
in-process Rust consumers.

## Forgejo HTTP is a native tool

`runseal :perish @forgejo` performs one named Forgejo HTTP resource operation
per invocation. The surface covers issues, pulls, reviews, commit status,
branches, protection, repository retirement, Actions secrets, workflow runs,
task perception, job logs, labels, and unauthenticated public reads. Lists
walk every page and may be capped with `--limit`.

The named profile is the credential seat. `@forgejo` reads `FORGEJO_URL` and
`FORGEJO_TOKEN_FILE` from the resolved profile and creates no login database.
`@forgejo --help` is the complete command map.

## One dialect, two entry points

The `runseal` crate is published to the `perish` Cargo registry alongside the
binary and skill. `runseal::tool::call` returns a structured `Reply`; the CLI
renders that result separately. A library consumer therefore uses the same
verbs and Forgejo dialect without parsing stdout or rebuilding routes.

Workflow task perception completes the HTTP evidence needed to distinguish
waiting, successful, failed, and blocked workflow outcomes in a caller.
