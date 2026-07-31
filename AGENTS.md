# AGENTS

## Product boundary

Runseal establishes per-run isolation by resolving one profile, applying it,
and executing one command process tree.

A profile has exactly three capabilities:

- `env`: set or remove child environment values.
- `argv`: insert fixed arguments after one matching command token.
- `symlink`: lease an otherwise absent target for the command lifecycle.

A profile carries no command, default action, task graph, hook behavior,
workflow, shell expression, or orchestration.

Runseal has two explicit planes:

- `runseal <internal-command>` operates the Runseal control plane.
- `runseal :[profile] <command|@tool> [args...]` enters profile mode. `:` uses
  the default profile and `:<name>` selects a named profile.

An unmarked unknown token belongs to the control plane and must quick-fail. The
colon is the only profile-mode signal.

## Tool boundary

An external CLI remains external when `env + argv + symlink` can establish its
per-run isolation. `kube`, `aws`, and `gh` are representative plumbable tools.

`@tool` is reserved for a third-party dependency that is not plumbable. It is a
native, atomic Runseal capability executed inside the selected profile. Do not
add aliases, convenience wrappers, task sequences, or broad SDK surfaces.

A perish.code-owned CLI that is not plumbable must be repaired at its own
boundary. A capability that stops being atomic should become a new product
closure.

Runseal has no Deno, Sealkit, Python, Node, shell-wrapper, or other scripting
runtime model.

## Configuration

Use Plumb's typed Cascade. Precedence is always:

1. total defaults;
2. the selected TOML file;
3. typed `RUNSEAL_*` environment values;
4. explicit Runseal arguments, when a field admits them.

Plumb owns the derive and doors. Runseal owns every profile field and name.
Product code does not invent environment keys or directly read configuration
from the process environment.

The default profile discovers `runseal.toml` from the current directory upward.
A named profile currently discovers `runseal.<name>.toml`. Explicit named
selection refuses when no file exists. The default profile is total and may be
empty when no file exists.

Profile TOML uses three top-level seats:

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

`resource://` resolves below committed `.runseal/resources`; `local://`
resolves below ignored `.local`.

## Runtime safety

- One invocation owns one child process tree.
- Profile argv is data-plane rewriting, distinct from Runseal's own Clap
  arguments.
- Profile context keys (`RUNSEAL_HOME`, `RUNSEAL_PROFILE`,
  `RUNSEAL_PROFILE_PATH`, and `RUNSEAL_ROOT`) are Runseal-owned.
- Symlink registration refuses every occupied target. Never replace, adopt, or
  silently repair an existing path.
- Symlink shutdown removes only the link whose source still matches the
  registered source.
- Do not add implicit shell evaluation, command lists, or background task
  ownership.

## Repository layout

- `app/src/bin/runseal.rs`: CLI plane selection and Clap control commands.
- `app/src/core/config.rs`: Plumb Cascade and profile discovery.
- `app/src/core/profile.rs`: Runseal-owned profile vocabulary and path
  normalization.
- `app/src/core/route.rs`: colon-mode routing.
- `app/src/runtime.rs`: argv resolution and external/tool dispatch.
- `app/src/injections/`: env patch and symlink lifecycle.
- `app/src/runner.rs`: one external process tree.
- `.runseal/resources/`: committed inert material.
- `.local/`: ignored secrets and local state.
- `plumb.toml`: release declaration consumed by stable Plumb.

Do not commit `.task/`, `.local/`, generated state, behavior files under
`.runseal/`, or a scripting-runtime lock/config.

## Validation

Work only on a dedicated task branch. During the Runseal-to-Plumb bootstrap,
the old Plumb doctor may report the intentional removal of filesystem wrappers;
record that seam and do not recreate wrapper evidence to make the old rule
green.

Run focused validation while iterating:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked --workspace --all-targets --release
cargo test --locked --workspace
ectropy .
```

Prefer small focused commits. A one-time `--no-verify` bootstrap commit is
permitted only when the installed hook invokes the obsolete runtime being
replaced; record the reason.

## Release

Stable Plumb owns binary build, archive, capsule, publication, activation,
inspection, smoke, and packport topology. Product workflows remain thin
callers. Do not publish or activate while the Runseal shape is still awaiting
the Plumb bootstrap update.

Stable is the canonical release authority plus the stable channel. Non-stable
releases use exact immutable versions and isolated install/bin paths. Stable
promotion requires the same product, base version, and commit as its exact
candidate.
