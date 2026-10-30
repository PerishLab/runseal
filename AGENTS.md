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

An external CLI remains external when the profile triad can isolate it. Native
`@tool` capabilities own typed provider protocols where that product boundary
is explicitly chosen; every operation must stay atomic. Feishu is such a native
capability, with the official lark-cli as its complete capability comparison
target and staged subsets as its delivery scope. A perish.code-owned CLI that cannot be isolated is
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

Private custody keeps secure opening, ancestor/owner/permission checks and state
updates in Runseal. File mutual exclusion delegates to Radicula. The public
`Private::lock` keeps its `File` return and legacy busy/error compatibility for
existing consumers such as Hardrig; it is a narrow owned-file RAII adapter.
Tencent and Feishu SDK sessions instead retain Radicula's owned-file Permit for the entire
Store lifetime, including early failure and unwinding. A session is not reduced
to one read/write closure, and no cloud operation follows automatically from
admission or release.

Feishu stage one uses an existing custom application's bot identity on fixed
mainland Feishu HTTPS endpoints. Private credential import, explicit Bearer
client_credentials issuance and current-token bot verification are separate
operations. No command automatically renews, retries, logs in, changes scopes
or falls back to another endpoint or authentication method. State is bound to
an explicit expected App ID; bot info does not prove a tenant, natural person,
resource ownership or permission.

`runseal::feishu::Client` exposes the same closed operations as `@feishu`.
Message get reads one exact message with open_id sender identifiers. Plain-text
reply sends one native reply with an explicit caller UUID and no hidden
pre-read. Its receipt is accepted, not proof of delivery or reading. Local
failures are unsent, documented provider refusals rejected, and ambiguous
transport/server/malformed/incomplete write results unknown. Errors never emit
provider prose or raw credentials. The caller owns durable intents, exact
target/content/UUID persistence, sender policy, pagination, event deduplication
and recovery. Platform UUID dedup lasts one hour; it is no durable exactly-once
contract. Do not replay unknown writes.

The comparison baseline for stage one is larksuite/cli v1.0.98 at
ac81f5056ab88f37d581d25916f3dab85abfca76. Its API catalog and shortcut families,
configuration/auth/profile/identity, schema/discovery/output, event and extension
surfaces remain the full comparison target. Stage one maps app credentials and
bot verification, exact message reading and the text subset of messages-reply.
It deliberately exposes exact reads rather than messages-mget's automatic
sender, reaction, thread and resource enrichment. The CLI grammar and JSON
receipt/error envelope are Runseal's, with no byte-compatible upstream claim.
User OAuth, DPoP, Lark brand, nontext replies, all other business domains and
compound shortcuts remain unimplemented. Raw/compound upstream capabilities
require explicit future correspondence with the typed/atomic product boundary,
not an implicit arbitrary gateway. Issue #99 retains the complete inventory and
mapping evidence. These are staged gaps, not a reduction of the long-term goal.

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

Network additionally reads one exact VPC resource dashboard, subnet resource
dashboard or security-group association statistic. Native verbs are vpc survey,
subnet survey and group associations. These invoke only their respective
Describe APIs; no pre-read, loop or deletion follows. Request::validate checks
finite input locally without custody or transport. Survey entries check exact
IDs and preserve parent scope, optional counts and per-product statistics.
Missing/null counts and arrays remain unknown; unknown product labels are
withheld and mark coverage incomplete. Unexpected fields also make coverage
incomplete. Group aggregate counters and instance statistics stay independent;
the SDK never infers a sum or treats TotalCount as including SG references.
Complete means coverage of the known response schema, not a distributed
idle lock or deletion permit. An empty survey remains unlisted; ResourceNotFound
is a safe diagnostic code, not an independent absence proof. Consumer ownership,
attachment policy, qualified negative observations and durable recovery remain
outside these atomic reads.

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

`runseal::tencent::cvm` and `Client::cvm` expose plain finite models shared by
the native CVM command. Existing compute/Lighthouse queries retain their public
paths and read contracts. Every CVM invocation requires explicit region, owner,
private custody and a current verified credential binding, and sends one API.
Request::validate performs local validation without opening custody or transport.

Creation and inquiry specify one POSTPAID_BY_HOUR instance: explicit zone,
project, VPC/subnet, image/flavor, system disk, security groups, imported key
and public bandwidth. The supported disks are CLOUD_PREMIUM/CLOUD_SSD, 10 through
4096 GiB; one through five unique groups and zero private-only or 1 through
100 public Mbps are finite closure limits, not proof of actual stock or quota.
There are no extra disks, arbitrary UserData, passwords, launch templates,
roles or raw API gateway. Cloud security, monitor and automation services are
explicitly disabled. LoginSettings contains KeyIds alone: KeepImageLogin is
mutually exclusive, including its string-valued false form, and is omitted.

RunInstances always includes a caller-supplied nonempty visible ASCII token
of at most 64 bytes and explicit DryRun. Persist the exact token/input before
sending; the SDK neither generates tokens nor promises a generic token lifetime.
DryRun success is Checked, never Created. A normal creation requires exactly
one returned InstanceId; accepted does not prove RUNNING, successful latest
operation, SSH reachability or application readiness. All remain separate facts.

InquiryPriceRunInstances does not receive DryRun or a token and performs no
purchase. Preserve hourly instance and per-GB traffic rates, optional discounted
rates and second/third pricing steps. Official step intervals are 0-96,
96-360 and 360 onward hours; absent later steps do not invent prices. The API
does not return a currency field: currency remains unknown, not an account
billing-currency proof or an approved spending budget. DescribeAccountQuota
returns checked postpaid zone counters; quota is not stock or purchase authority.

Exact instance/key/status reads check IDs and count coverage. A checked empty
result is an explicit unlisted observation, not an authoritative absence claim.
Missing fields, foreign zones, duplicate attachments and partial operation
facts refuse instead of becoming empty/success. Nullable addresses and latest
operation facts retain unknowns. Latest operation's SUCCESS/OPERATING/FAILED
and optional RequestId remain separate from instance state. Public facts retain
VPC/subnet/project, disks and deletion flags, groups, keys, IPs, bandwidth,
billing and termination protection; raw operation error messages are withheld.

Key import accepts only bounded OpenSSH RSA public material, 2048 through 4096
bits, with a positive odd exponent; names use at most 25 ASCII letters, digits
or underscores and project IDs fit the official signed integer. CLI reads a
bounded public file without printing its contents. It never generates a key,
uploads a private key or manages operator private custody. Key reads preserve
associated instance IDs; an accepted delete may still leave a referenced key.

Rename and terminate address one exact ID, with no hidden preflight or wait.
Terminate explicitly sets ReleaseAddress=false and ReleasePrepaidDataDisks=false;
this does not promise safety for every externally attached disk or address.
Consumers inspect current charge/attachments/ownership before destructive work.
Stopping may still bill and releasing a normal public IP loses that address;
these calls offer neither stop/free billing nor inventory recovery guarantees.
Hardrig owns plans, bindings, durable intents/receipts, generation, polling,
cost approval, SSH trust and recovery. Lost receipts never authorize replay,
compensation, automatic refresh or forced clearing. Protocol/offline tests are
not real Tencent account, stock, pricing, purchase or Windtun qualification.
