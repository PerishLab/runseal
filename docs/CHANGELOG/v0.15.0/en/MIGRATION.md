# Migrating to Runseal v0.15.0

This release adds the managed skill seat. Profile grammar is unchanged.

After stable is activated, install or upgrade the managed skill with the
stable binary:

```bash
runseal skill install
runseal skill upgrade
```

If an agent skill directory already exists at the default seat and is not in
the managed ledger, remove that directory, then install. `--force` cannot
claim it.

To evaluate a candidate before promotion, stage the exact non-stable skill
into an isolated path ending in `runseal`. Do not replace a managed stable
seat with a beta.

No profile, `runseal.toml`, or stored data requires migration.
