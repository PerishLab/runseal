# AGENTS

## 1. AGENTS.md Meta Constraints

This top-level `AGENTS.md` is the repository navigation and policy layer.

- Keep this file focused on shared constraints, navigation, and recurring
  operating guidance.
- Push local implementation detail downward into child `AGENTS.md` files when a
  directory starts carrying its own stable rules.
- Do not duplicate large bodies of module-specific instruction here once a child
  `AGENTS.md` exists.
- Treat this file as the default contract for the whole repository unless a
  deeper `AGENTS.md` overrides a narrower scope.
- Keep `.task/` out of git by default. Use it only for long-running work and
  update it as live task state, not as archival prose.

Core product stance:

- `runseal` is not just an operations toolkit. It is an operations methodology
  plus a derived tool system: the main value is deciding which operational
  complexity belongs in wrappers, atomic tools, repo/local artifacts, or
  external scripts, then keeping those layers explicit.
- `runseal` exists to reduce environment-dependency complexity in real
  cross-platform operations work: too many environment variables, too many
  machine-specific assumptions, and too much operational glue falling into
  uncontrolled Python or shell dependency stacks.
- Explicit profile. No hidden orchestration.
- Prefer Deno `.ts` wrappers for structured cross-platform operator flows,
  thin `.sh` wrappers only for Unix bootstrap glue, and explicit atomic `@tool`
  capabilities for reusable domain operations.
- Treat Deno as an explicit single-binary runtime prerequisite when the profile
  declares a `[deno]` policy. Do not hide runtime setup or prompt for missing
  permissions at wrapper execution time.
- Keep the Rust core thin and concrete.
- Support only `env`, `symlink`, fixed-prefix `argv`, explicit `:wrapper`
  resolution, Deno `.ts` wrapper execution, platform script wrappers, and
  read-only `@internal` introspection unless a new product decision explicitly
  expands the surface.
- Use `clap` for CLI parsing. Do not hand-roll argument parsing.
- Preserve command lifecycle semantics: load profile, register symlinks, export
  env, run command, clean up symlinks.
- Keep command namespaces explicit: `<cmd>` is external, `:<cmd>` is a profile
  wrapper, `@<cmd>` is runseal internal.

Runtime path rules:

- Treat `RUNSEAL_HOME` as the runseal configuration root.
- Treat `RUNSEAL_PROFILE_HOME` as the profile directory, defaulting to
  `<RUNSEAL_HOME>/profiles`.
- Resolve one concrete `RUNSEAL_PROFILE_PATH` during app initialization.

Tooling rules:

- Treat `runseal` and `ectropy` as installed developer infrastructure, at the
  same level as `git` and `cargo`; this repository does not bootstrap
  them.

## 2. Directory Conventions

Direct child directories with their own `AGENTS.md`:

- None yet.

Direct child directories that are likely future candidates for a child
`AGENTS.md` once their local rules become stable:

- `app/`: Rust application code, tests, and core runtime behavior.
- `.runseal/`: repo-local wrappers and operator-facing workflow glue.
- `.forgejo/`: CI, release automation, and workflow support scripts.
- `docs/`: durable operator or contributor documentation, if this area starts
  carrying rules distinct from code.

When a direct child directory gains its own stable constraints, add an
`AGENTS.md` there and link it from this section.

## 3. Core File Index

There are no child `AGENTS.md` targets yet, so this index currently points to
the repository-owned canonical files directly.

- `app/src/bin/runseal.rs`: CLI entrypoint.
- `app/src/core/config.rs`: app configuration and profile discovery.
- `app/src/core/profile.rs`: profile format loading and normalization.
- `app/src/core/runtime.rs`: command execution lifecycle.
- `app/src/injections/`: `env` and `symlink` implementations.
- `app/src/tool/`: built-in atomic `@tool` surface.
- `app/tests/`: integration tests and focused behavioral coverage.
- `.runseal/wrappers/`: repo-local `:wrapper` entrypoints. Prefer `.ts`
  wrappers for structured operations and `.sh` only for thin Unix bootstrap.
- `runseal.toml`: repo-local operator profile.
- `plumb.toml`: product release declaration consumed by stable Plumb.
- `.forgejo/workflows/release-exact.yml`: immutable non-stable publication.
- `.forgejo/workflows/release-stable.yml`: stable promotion and activation.

Once child `AGENTS.md` files exist, this section should prefer links to those
local guides over repeating their detail here.

## 4. Daily Iteration Workflow And Commands

Normal workflow:

1. Work on a dedicated feature branch for every substantive change.
2. Keep changes scoped to the current product boundary.
3. Prefer `git commit` as the default validation trigger; the repo-installed
   pre-commit hook runs `runseal :guard`.
4. Use standalone lint, format, or test commands only for focused diagnosis or
   repair after the repo guard reports a failure.
5. Use repo wrappers for recurring operator flows when they already encode the
   intended path.

Branch safety:

- The generated pre-commit hook rejects direct commits on `main`; create a
  feature branch before committing.
- Treat `--no-verify` as an explicit exception only. If used, record why in the
  surrounding handoff or final note.

Default validation path:

```bash
git commit
```

Use `runseal :guard` only when an explicit manual guard run is needed before a
commit or while diagnosing hook failures.

Common repo workflow commands:

```bash
runseal :init
runseal :cloudflare
runseal :land
```

Canonical stable install path:

```bash
curl -fsSL https://releases.runseal.perish.uk/manage.sh | sh
```

Release and distribution rules:

- Stable Plumb owns the complete binary release mechanism. This repository
  declares its product in `plumb.toml`; its two release workflows are thin
  Actions callers.
- Every release produces immutable content-addressed objects and one exact seal
  at `v1/releases/<channel>/<version>/seal.json`.
- Non-stable releases stop at their exact seal. Their generated managers carry
  the exact channel and version and require explicit install/bin paths isolated
  from the stable defaults.
- Stable is promoted from a verified exact non-stable seal for the same commit.
  Only stable activation may update `v1/channels/stable.json` and the canonical
  root `manage.sh` and `manage.ps1`.
- Release jobs resolve the requested ref once and use that immutable commit for
  build, publication, smoke, and stable tagging.
- Publication and activation use separate credential sets.
- Generated managers and release capsules are release outputs, never
  repository-owned source files.

Profile discovery order:

1. `--profile <path>`
2. From `<cwd>` upward to filesystem root, at each directory:
   - `runseal.toml`
   - `runseal.yaml`
   - `runseal.yml`
   - `runseal.json`
3. `<RUNSEAL_PROFILE_HOME>/default.toml`
4. `<RUNSEAL_PROFILE_HOME>/default.yaml`
5. `<RUNSEAL_PROFILE_HOME>/default.yml`
6. `<RUNSEAL_PROFILE_HOME>/default.json`

Format priority is TOML, YAML, then JSON within each searched directory.
Successful profile and wrapper paths are normalized absolute paths.

## 5. FAQ

### What defines the CLI surface?

This repository is building explicit runtime glue, not a hidden orchestrator.
New behavior should be added only when it fits one of these shapes cleanly:

- a Deno `.ts` wrapper for repo-local structured operational flow
- an explicit atomic `@tool`
- a thin platform script for bootstrap or platform-specific shell integration

`runseal` should not be treated as a grab-bag operations toolkit where every
pain point becomes another command. Its value is methodological first:

- decide what should be flow control in a `.ts` wrapper
- decide what should be an atomic `@tool`
- decide what should be a visible repo or local artifact under `.runseal/` or
  `.local/`
- decide what should remain an external script because it carries the wrong
  kind of complexity

The concrete tools matter, but they are derived from that layering model rather
than the other way around.

This boundary comes from the actual problem `runseal` is trying to solve:
clear operational workflows should not need to depend on heavyweight language
runtimes or repository-local script stacks just to survive environment drift,
cross-platform differences, and routine operator setup friction.

The goal is not "no runtime dependencies ever". The goal is to absorb the
right kind of complexity with explicit prerequisites:

- clear, finite, cross-platform operational flow control should fit in Deno
  wrappers plus runseal profile/context glue
- reusable domain operations should become `@tool`
- shell-specific cleverness, open-ended scripting power, and accidental
  dependency sprawl should not

That is why the product boundary is Deno-first wrappers plus explicit atomic
tools, rather than a general scripting platform or a partial shell clone.

### When should behavior become a Deno wrapper?

When the logic is repo-local operational flow: argument parsing, policy,
defaults, validation, polling, JSON/HTTP handling, and sequencing around
existing CLIs or runseal tools.

### When should behavior become `@tool`?

When native CLI coverage is insufficient for an atomic, reusable operation and
the result still fits the explicit atomic-tool model.

### When should logic stay outside runseal?

When the behavior cannot be described cleanly as a repo-local Deno flow or a
clear atomic tool, keep it in Python, Ruby, JavaScript, Zig, shell, or another
external script.

### Should wrappers build multi-line config or payload text inline?

Usually no.

For operations work, persistent or semi-persistent structured text should
normally live as explicit repo material under `.runseal/` or `.local/`, not as
inline heredoc-style wrapper content. That includes things like:

- config templates
- YAML or JSON fragments
- kube-related files
- long request bodies
- other operator-facing text payloads

The wrapper should usually do the smaller, clearer job:

- validate preconditions
- choose the right file or template
- assemble paths and arguments
- set environment for the invoked command
- execute the operational flow

This is an intentional product boundary. `runseal` is meant to reduce
environment and runtime dependency complexity in operations workflows, not to
turn wrappers into a general inline text-construction language. If a multi-line
artifact is important enough to exist, prefer making it a visible repo or local
artifact first.

### Should `.ts` wrappers be treated as first-class runtime entrypoints?

Yes. Treat `.runseal/wrappers/*.ts` as first-class wrappers executed by runseal
through the selected profile's `[deno]` policy.

### What should never be committed?

- `.task/`
- accidental broad surface expansions that were not backed by an explicit
  product decision

### What is the commit style?

Prefer small focused commits.

## Release

- Stable is the consensus anchor: its root managers and
  `v1/channels/stable.json` are the only moving public surfaces.
- Every other channel is addressed by an exact seal and installed into an
  explicit isolated seat with the generated manager named by that seal.
- A stable release requires
  `docs/CHANGELOG/v<version>/{en,zh}/{INDEX.md,MIGRATION.md}`, enforced by the
  `Changelog` step in `release-stable.yml` before anything irreversible.
  `plumb doctor` does not check this: a changelog is owed by a release, not by a
  working tree. A release requiring nothing of anyone still writes MIGRATION.md
  saying so. See `plumb/docs/changelog.md`.
