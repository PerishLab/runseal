# Agents

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure. That document owns
organization-wide Issue, pull-request and acceptance policy; this file keeps
repository-specific constraints without copying that policy.

Runseal establishes one profile around one command process tree. It owns
isolation, not orchestration: a profile has exactly `env`, `argv`, and `symlink`
and carries no command, default action, wrapper, task graph, hook, shell, or
scripting runtime.

The colon is the only profile-mode signal. An unmarked unknown token belongs to
Runseal's control plane and must quick-fail. Profile argv is data-plane rewriting
and never changes Runseal's own arguments.

An external CLI remains external when the profile triad can isolate it. `@tool`
is reserved for a third-party dependency that cannot be isolated and every
operation must stay atomic. A perish.code-owned CLI that cannot be isolated is
repaired at its own boundary; a capability that stops being atomic becomes a
new product.

External preparation reports at most one source-less `runseal.exec=prepared.v1`
Atom after profile argv resolution, before projection or spawn. It promises no
child startup, effective environment, internal subprocess or session identity.
The CLI owns this vocabulary; the library exposes preparation without an
observer dependency or callback, and its existing `run` entry still prepares
and executes exactly once.

Only the preparation Atom binds `runseal.program`, `runseal.target` and
`runseal.wrapper`. Programs are the literal public tokens `gh`, `concord`,
`plumb`, `env` or `other`; a target is recognized only directly or after one
simple env assignment prefix. Unknown syntax or paths stay opaque. Payload is
the ordered tuple `[overrides, program, target, verb]`, containing only finite
public constants. It retains only the exact explicit assignment
`GODEBUG=http2client=0`; every other assignment becomes `other`, with neither
its name nor value retained. No process environment is collected.

Normalization inspects at most 32 argv tokens of at most 128 UTF-8 bytes each;
excess yields one fixed opaque shape. At most 30 assignment markers and one
allowlisted verb enter the payload. Serialized preparation Atoms stay under
4096 bytes; product identity roles have finite cardinalities (exec 1,
program 5, target 4, wrapper 3), independent of argv, profile names or paths.
Observation remains opt-in and cannot alter execution. Inspect with an exact
execution/wrapper selector in a bounded producer window, then query the records
before interpreting a repeated prefix. Ordinary repeated calls can also match.

Cloudflare operations expose typed provider resources without
exposing credentials, raw response secrets, desired-state bundles, or
orchestration. One invocation owns one provider verb or one child process tree.

The reusable library carries no Plumb dependency. The CLI owns the product's
managed Skill surface while delegating Depot mechanics to Plumb. Runseal has no
Cookbook: its routine refusals state their own recovery.
