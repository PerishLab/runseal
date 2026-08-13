---
name: runseal
description: Operate Runseal, the per-run profile isolation tool. Use when inspecting or applying a profile, editing a runseal.toml, or changing the Runseal repository.
---

# Runseal

Runseal resolves one profile, applies it, and executes one command process
tree. Use this brief when inspecting or applying a profile, editing a
`runseal.toml`, or changing the Runseal repository.

## Objects

- A **profile** has exactly `env`, `argv`, and `symlink`. It carries no
  command, default action, wrapper, task graph, hook, shell, or orchestration.
- The **control plane** is `runseal <internal-command>`. An unmarked unknown
  token belongs here and must quick-fail.
- **Profile mode** is `runseal :[profile] <command|@tool> [args...]`. `:`
  selects the default profile and `:<name>` selects a named profile. The colon
  is the only profile-mode signal.
- **`env`** sets or removes child environment values.
- **`argv`** inserts fixed arguments after one matching command token.
- **`symlink`** leases an otherwise absent target for the child lifecycle.
- **`@tool`** is reserved for a third-party dependency that cannot be isolated
  by the profile triad. It must stay one atomic operation. `@forgejo` and
  `@cloudflare` each perform one provider HTTP resource verb per invocation.

## Actions

```bash
runseal profile [NAME]
runseal resolve [--profile NAME] URI...
runseal : <command> [args...]
runseal :<name> <command> [args...]
runseal :<name> @forgejo <resource> <verb> ...
runseal :<name> @cloudflare <resource> <verb> ...
runseal skill --help
```

The binary is the authority for flags. Use `runseal <command> --help` before
an unfamiliar action.

## Operating laws

- One invocation owns one child process tree. Do not add command lists,
  implicit shell evaluation, or background task ownership.
- An external CLI remains external when `env + argv + symlink` can isolate it.
- A perish.code-owned CLI that cannot be isolated must be repaired at its own
  boundary. A capability that stops being atomic becomes a new product.
- Runseal has no scripting runtime model.
- Cascade precedence is defaults, then the selected TOML file, then typed
  `RUNSEAL_*` values, then explicit arguments when a field admits them.
  Runseal owns every profile field and name. Product code does not invent
  environment keys or read the process environment directly.
- The default profile discovers `runseal.toml` from the current directory
  upward and may be empty. A named profile discovers `runseal.<name>.toml`
  upward, then `$RUNSEAL_HOME/profiles/{name}.toml`, then
  `$RUNSEAL_HOME/profiles/{name}/runseal.toml`, and refuses when none exist.
  A flat home file wins when both home writings are present.
- `resource://` resolves below committed `.runseal/resources`. `local://`
  resolves below ignored `.local`.
- Profile context keys `RUNSEAL_HOME`, `RUNSEAL_PROFILE`,
  `RUNSEAL_PROFILE_PATH`, and `RUNSEAL_ROOT` are Runseal-owned.
- Symlink registration refuses every occupied target. Shutdown removes only
  the link whose source still matches the registered source.
- Profile argv is data-plane rewriting, distinct from Runseal's own
  arguments.

Use [PATHS.md](PATHS.md) for routine flows and [SCENARIOS.md](SCENARIOS.md)
only when one of its bounded cases applies.
