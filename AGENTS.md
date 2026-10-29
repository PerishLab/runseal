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

CAM refusals retain a closed set of documented nonsecret error codes and a
validated RequestId in typed failures and CLI errors. Unknown codes and all
provider messages remain withheld; refusal alone does not justify replaying a
write. Inspect the exact resource before changing a failed enrollment intent.

The finite `runseal::tencent::network` namespace supplies typed VPC, Subnet and
SecurityGroup operations through the same `Client` and custody. `Client::network`
and `@tencent network --region REGION` share their request construction and
result decoding. The library still carries no Plumb dependency. The private
command module groups existing CLI CAM/DNS grammar; the common client is no
longer housed under the compute capability. Existing public compute types and
methods retain their contracts.

Network operations select one create, exact-ID get, one-page list, supported
metadata update or delete. Security groups additionally read the complete
policy set or reset both ordered ingress and egress with a nonzero observed
version. Reset calls ModifySecurityGroupPolicies with SortPolicys=true;
ReplaceSecurityGroupPolicies is the distinct indexed-rule API and is outside
this closure. Version zero refuses because it clears all policies and ignores
the supplied arrays. The SDK never invents a ClientToken for network creation,
performs an implicit read-before-write, loops pages, waits or compensates.

A rule is a typed TCP/UDP/ICMP/ALL IPv4 selector and ACCEPT/DROP action, with
optional public description. The CLI spells each repeated ingress/egress rule
as protocol;ports;cidr;action with optional ;description; use a quoted single
argument, '-' for omitted ICMP/ALL ports, and 'all' for all TCP/UDP ports.
A reset always submits both arrays; an omitted direction means an empty owned
set. Active address/service templates, referenced groups, IPv6 and other
protocols refuse the complete policy observation. Empty template metadata is
not an active selector. Descriptions and rule order survive typed decoding.
No arbitrary API/body/endpoint, shared rule-writer model or expression syntax
is exposed.

A network receipt carries verified credential binding, requested region,
service/action/version, safe RequestId, observation time and finite data.
Create and other mutation acknowledgements are accepted, not proof of final
resource state. Exact reads reject a different native ID; empty pages, missing
coverage, duplicate IDs and nonprogressing pagination remain distinguishable.
Known current-caller preflight failures are unsent, allowlisted provider
refusals are rejected, and transport/malformed/incomplete/unknown write results
remain unknown. CAS conflict preserves its documented safe code. These statuses
never authorize automatic replay. The consumer owns Binding, LastManaged,
intent/receipt durability, operation ordering, budgets, final exact reads and
recovery. A successful protocol fixture does not prove cloud permissions,
quota, current pricing, resource creation, final convergence or clean deletion.

`runseal::tencent::dns` exposes plain public Request/Operation/Zone/Fields and
result types, independently of CLI grammar. `Client::dns` uses the existing
private custody and verified caller binding for every read or write. Native
DNS commands retain domain/record list/create/modify/delete and their flags;
record get adds exact DescribeRecord. A command selects exactly one --domain
or --domain-id, never both. Domain qualification is one explicit DescribeDomain
operation; the SDK does not perform a hidden lookup before a record request.
All DNSPod calls use the fixed global endpoint/version without X-TC-Region.

CLI DNS output cuts over to the typed receipt instead of the earlier
{action,result} projection. All DNS reads now require an explicit owning
account and verified current credential binding, as compute/network already
do; run identity explicitly first. No private state migration, automatic
renewal or compatibility fallback follows. The public write kinds remain A,
AAAA, CNAME and TXT. Observations also retain other bounded record kinds and
system/default NS entries so a zone page cannot silently drop foreign records.
Public record values and metadata must never carry credentials.

A complete DNS locator has account, nonzero DomainId and RecordId. A named
list or mutation acknowledgement that lacks DomainId retains an explicitly
partial Reference; complete refuses it. Qualify the domain, then select its
ID for complete record references. Exact reads verify the returned record and
domain IDs. Qualified domain facts retain optional Tencent Uin separately from
DNSPod UserId; missing Uin is unknown. Caller identity and a complete locator
are not themselves a domain-ownership proof; consumers qualify owner/scope
before binding or mutating shared domains.

Lists set ErrorOnEmpty=no and return one checked page with offset/limit/total/
next; limits are 1 through 3000. Count mismatch, duplicates, missing facts and
nonprogress refuse. A known empty index page is not proof of record absence:
DNSPod documents transient create indexing delay, with requery after about
30 seconds. Exact DescribeRecord is a separate operation. Identifier/permission
refusals remain errors with finite safe codes; RecordIdInvalid alone does not
prove absence. No selected mutation claims CAS, ClientToken, distributed locking,
whole-zone reconciliation or automatic replay. Accepted mutation receipts need
subsequent qualified observations before a consumer can claim convergence.

Error/Outcome now live at the common Tencent API boundary; existing public
network aliases retain the same data contract. The common identity capability owns caller verification, bindings, private State
and credential custody. CLI authorization ceremonies live under command. Public
compute Product/Window/Query remain plain data models; command owns clap parsing
and explicit conversion. State v1 and existing public type paths remain stable. No Plumb
or Locus dependency enters the reusable library.
