# Runseal v0.14.1

This is the first stable release of the 0.14 line. The v0.14.0 exact rehearsal
stopped at the obsolete wrapper-based source guard and was never published.

## One profile, one process tree

Runseal now has one core closure: resolve a profile, establish per-run
isolation, and execute one command process tree. A profile contains exactly
three capabilities:

- `env` sets or removes child environment values;
- `argv` inserts fixed arguments after a matching command token;
- `symlink` leases an otherwise absent target for the child lifecycle.

Profiles carry no command, default action, task graph, hook behavior, shell
expression, or orchestration.

## The colon enters profile mode

No-colon invocations stay on Runseal's control plane and unknown commands
quick-fail through Clap. `:` selects the default profile, while `:<name>`
selects a named profile:

```bash
runseal profile
runseal : cargo test --locked
runseal :release cargo publish
```

Profile mode requires one external command or one native `@tool`. No native
tools ship in this release: external CLIs remain external whenever
`env + argv + symlink` can isolate them.

## Conventional material

Committed inert resources live under `.runseal/resources`; ignored secrets and
local state live under `.local`. `resource://` and `local://` profile values
resolve into those seats with traversal-resistant paths.

Runseal no longer hosts Deno, Sealkit, filesystem wrappers, repository hooks,
or any replacement scripting runtime.
