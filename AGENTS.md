# Agents

This repository is the Runseal product. An agent changing it holds the same
closure the binary enforces on callers: one profile, one process tree, no
wrapper or scripting runtime.

## Laws

- A profile has exactly `env`, `argv`, and `symlink`. It carries no command,
  default action, wrapper, task graph, hook, shell, or orchestration.
- An unmarked unknown token belongs to the control plane and must quick-fail.
  The colon is the only profile-mode signal.
- An external CLI remains external when the profile triad can isolate it.
  `@tool` is reserved for a third-party dependency that is not isolatable, and
  must stay atomic. `@forgejo` is the first native tool.
- A perish.code-owned CLI that is not isolatable must be repaired at its own
  boundary. A capability that stops being atomic becomes a new product.
- Runseal has no Deno, Python, Node, shell-wrapper, or other scripting runtime
  model.
- Do not commit `.task/`, `.local/`, generated state, behavior files under
  `.runseal/`, or a scripting-runtime lock/config.

## Layout

- `app/src/bin/runseal/main.rs`: CLI plane selection and Clap control commands.
- `app/src/bin/runseal/skill.rs`: managed skill seat.
- `app/src/core/config.rs`: cascade and profile discovery, including home
  `profiles/{name}.toml` and `profiles/{name}/runseal.toml`.
- `app/src/parse.rs`: shared `@tool` argv line and output envelope.
- `app/src/http.rs`: generic HTTP send.
- `app/src/tool/`: native `@tool` adaptors. `@forgejo` is the first.
- `app/src/core/profile.rs`: profile vocabulary and path normalization.
- `app/src/core/route.rs`: colon-mode routing.
- `app/src/runtime.rs`: argv resolution and external/tool dispatch.
- `app/src/injections/`: env patch and symlink lifecycle.
- `app/src/runner.rs`: one external process tree.
- `skills/runseal/`: operating brief.
- `.runseal/resources/`: committed inert material.
- `.local/`: ignored secrets and local state.

## Operating

- Never work or commit in the clean `main` integration checkout.
- Work only on a dedicated task branch.
- Before landing, run `cargo fmt --all --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo check --locked --workspace --all-targets --release`,
  `cargo test --locked --workspace`, and `ectropy .`.
- Skill content ships through the shared skill mechanism. Runseal owns its
  vocabulary and standing; packaging and managed placement are derived.

## Release

- This repository declares the product and skill in `plumb.toml`; its
  exact/stable workflows are thin callers.
- Every release produces immutable content-addressed objects and one exact
  seal. Non-stable releases stop at that seal and install only into explicit
  isolated paths. Stable promotion proves an exact non-stable seal from the
  same commit.
- Generated managers and capsules are release outputs, not repository files.
- Managed skill install, status, and upgrade are stable-only. Exact
  non-stable briefs use `skill stage` at a new explicit path and never enter
  the ledger.
- A stable release requires
  `docs/CHANGELOG/v<version>/{en,zh}/{INDEX.md,MIGRATION.md}`. A working tree
  does not owe a changelog. Follow the release-local contract under
  `docs/CHANGELOG`.
