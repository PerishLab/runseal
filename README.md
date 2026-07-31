# Runseal

Runseal establishes one profile for one command process tree.

```text
profile = env + argv + symlink
```

It is not a task runner, wrapper host, scripting platform, shell, or secret
manager.

## Command model

Runseal has two planes:

```bash
runseal <internal-command>
runseal : <command> [args...]
runseal :profile <command> [args...]
runseal :profile @tool [args...]
```

No colon means the normal Runseal control plane. An unknown internal command
quick-fails naturally through Clap.

The colon enters profile mode. `:` selects the default profile;
`:release`, for example, selects the named `release` profile. Profile mode
requires exactly one external command or one native `@tool` invocation.

## Profiles

The default profile discovers `runseal.toml` from the current directory upward.
When no file exists, the total default is an empty profile. A named profile
discovers `runseal.<name>.toml` and refuses when it is absent.

Profiles use Plumb's typed configuration cascade: total default, TOML file,
typed environment, then admitted arguments.

```toml
[env]
unset = ["AMBIENT_KEY"]

[env.vars]
KUBECONFIG = "local://secrets/kube/config"
TEMPLATE = "resource://template.yaml"

[argv]
kubectl = ["--request-timeout=30s"]

[[symlink]]
source = "resource://tool.conf"
target = "local://run/tool.conf"
```

`resource://path` resolves to
`<repository>/.runseal/resources/path`. `local://path` resolves to
`<repository>/.local/path`. URI paths reject traversal and ambiguous segments.

Environment values are applied only to the child process. Argv values are
inserted immediately after an exact matching command token. Symlinks are
created immediately before execution and removed afterwards; an occupied
target is always refused.

## Tools

An external dependency needs no Runseal implementation when the profile triad
can isolate it. Tools such as Kubernetes, AWS, and GitHub CLIs remain ordinary
external commands.

`@tool` is the narrow escape hatch for a third-party dependency that cannot be
made per-run isolated through env, argv, and symlink. A tool is native,
atomic, versioned with Runseal, and receives the resolved profile context.

First-party perish.code CLIs do not use `@tool` to hide an isolation defect;
they are repaired at their own boundary. A capability that grows beyond an
atomic operation becomes a separate closure.

## Control plane

```bash
runseal profile
runseal profile release
runseal resolve resource://template.yaml local://secrets/token
```

`profile` resolves and validates a profile without applying it. `resolve`
prints conventional material paths without executing profile behavior.

## Development

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked --workspace --all-targets --release
cargo test --locked --workspace
ectropy .
```

The current feature line intentionally precedes Plumb's removal of
filesystem-wrapper rules. Until that downstream bootstrap lands,
`plumb doctor` reports the known shape transition.

Repository: https://git.perish.top/PerishFire/runseal
