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

Tencent operations use the official Tencent Cloud CLI account authorization
application through native HTTPS, without Python. Browser login, MFA and
consent remain human actions. A private return file is consumed after durable
local custody; credentials never enter command arguments or provider output.
Refresh, credential renewal and identity verification are separate operations.
Local logout does not promise remote revocation. Mutations bind a verified
caller to an explicitly declared owning account; key issuance additionally
requires an explicit delegated user and a reserved private destination.
An uncertain provider result is never replayed automatically. The finite CAM
and DNSPod surface exposes no raw action or endpoint override. Source protocol
tests do not establish real account permissions or authorization compatibility.

The reusable library carries no Plumb dependency. The CLI owns the product's
managed Skill surface while delegating Depot mechanics to Plumb. Runseal has no
Cookbook: its routine refusals state their own recovery.

`runseal::tencent::Client` opens one absolute private state reference with an
explicit owning account. `identity` verifies the current caller and records the
credential stamp and observation time; `read` consumes that binding and one
closed `Query`. Its typed result carries product, service/version/action,
requested region, caller identity, observation time, RequestId and one data page
or catalog response. Cached identity time remains visible; a caller needing
fresh evidence explicitly calls identity first. No call renews, enrolls or
creates an identity implicitly. The existing tool namespace delegates here.

`@tencent compute` exposes the same instances, regions, zones, shapes, images,
blueprints and bundles queries. Each invocation requires the expected account
and verified current credential binding, performs one named API, and returns
one page. The operator owns pagination, budgets and infrastructure semantics.
Region discovery uses the region service; compute calls require an explicit
region. Image reads select CVM public images or Lighthouse pure OS blueprints.
Bundle reads may name one blueprint for compatibility filtering. Unknown,
missing, malformed, cross-product/region or nonprogressing page evidence refuses
rather than becoming an empty collection. Typed failures distinguish input,
custody, binding, authentication/expiry, permission, network, limits, response,
unsupported and other provider refusals without exposing raw provider messages.
