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
is reserved for a third-party dependency that cannot be isolated, with Feishu
explicitly established as a native capability by Issue #99. Its complete
capability target is official lark-cli, delivered in staged subsets. Every
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

Private custody keeps secure opening, ancestor/owner/permission checks and state
updates in Runseal. File mutual exclusion delegates to Radicula. The public
`Private::lock` keeps its `File` return and legacy busy/error compatibility for
callers that deliberately tie mutual exclusion to physical File descriptors.
Descriptor copies can retain that physical lock after the original File closes.
Public `Private::hold` opens the same protected file and returns Radicula's
owned-file Permit with immediate admission. Consumers retain that Permit for
the whole session; Drop, explicit release and unwinding end the held interval
even while an unrelated descriptor copy remains alive. They never unlock the
underlying file independently. A bare descriptor copy is not a session permit.
Tencent and Feishu SDK sessions use this same held interval for the entire
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

Permission refusals retain only recognized static subjects from structured
permission violations, with unknown entries and truncation explicit. No prose,
descriptions or help URLs are exposed. Subjects alone establish neither an
all-required grant set nor whether application settings are effective.

The comparison baseline for stage one is larksuite/cli v1.0.98 at
ac81f5056ab88f37d581d25916f3dab85abfca76. Its API catalog and shortcut families,
configuration/auth/profile/identity, schema/discovery/output, event and extension
surfaces remain the full comparison target. Stage one maps app credentials and
bot verification, exact message reading and the text subset of messages-reply.
It deliberately exposes exact reads rather than messages-mget's automatic
sender, reaction, thread and resource enrichment. The CLI grammar and JSON
receipt/error envelope are Runseal's, with no byte-compatible upstream claim.
Issue #105 adds Bearer device authorization, explicit single exchange/refresh,
user_info verification, local user logout, plain-text send and exact-chat
single-page list. User identity requires an explicit expected open_id in
addition to the App ID; profile names never authenticate people.
An explicit --as overrides the declared profile FEISHU_ACTOR; absent both, bot
remains the default. Status names the selected actor and expected person. Bot and user
credentials/bindings remain separate. Requested scopes are explicit and must
include offline_access; no automatic grant expansion or browser action occurs.
OAuth token-changing dispatch reserves an uncertain local state before network
access. Only documented pending/slow-down responses permit the next explicitly
scheduled exchange; unknown exchange/refresh results require observation and
explicit local discard/reauthorization if unrecoverable. Local logout does not
revoke remote credentials. State upgrades to version two when user authorization
begins; older binaries refuse it instead of silently dropping user custody.

Token returns received as successful OAuth JSON are retained in private custody
before credential-field validation, with their original request start time and
safe request ID. A retained return advances state to version three; older
binaries refuse it. Status exposes only bounded shape/length/count metadata;
raw tokens and provider prose never leave custody. Explicit local auth recover
revalidates that same return without network access or extending its lifetime,
then requires separate identity verification. Missing or unreadable returns
remain unknown and unrecoverable locally; no exchange/refresh replay follows.
CLI help and finite diagnostic fields own routine operation/recovery guidance.
Requested scopes remain caller supplied and limited to 32 entries. Granted
scope metadata is independent: accept up to 512 entries and 16 KiB of scope
names, retaining every normalized grant rather than truncating to the request.
Each name remains at most 160 bytes with the same finite character grammar.
Observed grants never expand supported operations or the consumer's authority.

Message sends require an explicit chat or open_id target and caller UUID;
user write acknowledgements must match the verified person, bot writes the
verified application/bot. Lists read one explicit chat and preserve cursor and
has-more evidence, rejecting foreign chats, duplicate IDs and nonprogressing
pages. They never resolve peers, enrich or paginate implicitly. Consumers own
verification workflows and durable dispatch/result records, including finding
an exact bot response to an exact user query.

DPoP, Lark brand, nontext messages, all other business domains and compound
shortcuts remain unimplemented. Raw/compound upstream capabilities
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

Instance address decoding
selects CVM PublicIpAddresses/PrivateIpAddresses or Lighthouse
PublicAddresses/PrivateAddresses by the requested product. Foreign product fields
refuse, even when null or empty. Native missing/null remains unknown and native
empty arrays remain observed empty; malformed addresses refuse. The normalized
public/private output and public Instance DTO retain their existing shape.

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
result types, independently of CLI grammar. `Zone` carries the required
canonical domain name and an optional nonzero DomainId. `Client::dns` uses the
existing private custody and verified caller binding for every read or write.
Native DNS commands retain domain/record list/create/modify/delete and their
flags; record get adds exact DescribeRecord. Every current DNS command requires
--domain. Optional --domain-id sends that DomainId with the name. Id-only input
is refused. DescribeDomain, DescribeRecord, DescribeRecordList, CreateRecord,
ModifyRecord and DeleteRecord all use this shape. Official schemas require
Domain and treat DomainId as optional and overriding: when both are present the
provider uses DomainId and ignores Domain. Supplying both is not an ownership
or name-mapping proof. Callers qualify that correspondence separately. The SDK
performs no hidden lookup, retry, fallback or extra read. All DNSPod calls use
the fixed global endpoint/version without X-TC-Region.

CLI DNS output cuts over to the typed receipt instead of the earlier
{action,result} projection. All DNS reads now require an explicit owning
account and verified current credential binding, as compute/network already
do; run identity explicitly first. No private state migration, automatic
renewal or compatibility fallback follows. The public write kinds remain A,
AAAA, CNAME and TXT. Observations also retain other bounded record kinds and
system/default NS entries so a zone page cannot silently drop foreign records.
Public record values and metadata must never carry credentials.

A complete DNS locator has account, nonzero DomainId and RecordId. Name-only
list pages and mutation acknowledgements do not carry DomainId and retain an
explicitly partial Reference; complete refuses it. Exact DescribeRecord returns
DomainId and RecordId, so that returned identity can complete a name-only read.
A supplied DomainId that differs from the returned record or qualification
DomainId refuses, as does a qualification name that differs from the supplied
name. The receipt keeps the request name and DomainId; response facts stay on
the data. Qualified domain facts retain optional Tencent Uin separately from
DNSPod UserId; missing Uin is unknown. Caller identity, a name plus DomainId,
and a complete locator are not a domain-ownership or name-mapping proof.
Consumers qualify owner, scope and that name/ID correspondence before binding
or mutating shared domains.

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

## Instance network attachments

`network interfaces --instance INS --offset N --limit N` and the public
`network::Operation::Interfaces` issue one DescribeNetworkInterfaces query with
only the exact attachment.instance-id filter. The limit is 1 through 100. This
is an observed page, not a CVM absence proof, full account inventory, attachment
ownership grant or deletion permit. The consumer owns pagination, cross-page
coverage, policy, race handling and subsequent mutation decisions.

Each entry requires a unique native ENI ID, VPC/subnet identity and an attachment
matching the requested instance. Available zone facts must match the requested
region. Nullable owner/device/primary/type/status/protection/groups and address
collections remain unknown. AddressId and public address retain explicit empty
strings separately from missing/null. Public IPv4/IPv6, extra addresses and EIP
IDs remain visible; unknown native labels are bounded, not translated to idle.

Only the finite public projection leaves the SDK. Tags, names, descriptions and
raw provider JSON are withheld. Unknown response keys or unprojected nonempty
CDC/legacy IPv6 facts mark schema coverage incomplete. Nested address coverage
propagates to the interface. Schema coverage is not completeness of all nullable
facts, a distributed lock or evidence that termination is safe. No interface or
address create/delete/detach/release action is introduced, and no read or retry
follows automatically. Existing custody, identity, signing, redaction, outcome
and local request validation remain shared with the network capability.

Feishu reaction create and exact-message one-page list are independent atomic
SDK and CLI operations. Emoji names are case-sensitive (`Get` is the platform's
GET acknowledgment). A create acknowledgement must match the verified actor and
requested emoji; it is accepted, not delivery proof. Reaction IDs are opaque.
Lists preserve message scope, cursor and has-more evidence without enrichment.
No pre-read, automatic pagination, retry, UUID dedup or delete compensation is
provided. Consumers persist intents before dispatch, observe unknown writes
explicitly, and own webhook event dedup and workflow policy. User scope grants
do not prove bot permission. Issue #114 owns this subset and its evidence.

Feishu message search adds one POST /im/v1/messages/search page from the same
fixed lark-cli baseline, without its compound body/chat/reaction enrichment.
Client::search and im messages search take an optional keyword (empty allowed),
explicit ordered Unix-second start/end, page size 1..50 and optional cursor.
The request converts times to UTC RFC 3339; receipts retain message IDs in
provider order, verified actor, request ID, has-more and next cursor. Missing
coverage, duplicate/invalid IDs and nonprogressing pages refuse. Search results
are indexed observations, not complete chat history or proof of absence.
Consumers own exhaustive paging, exact message/history reads, time and sender
verification, exclusions and resource access policy. No refresh, scope grant,
peer resolution, body lookup or retry follows. User grants never prove bot
access. Issue #126 owns this typed atomic subset and its release evidence.

`runseal::tencent::tat` and `Client::tat` share two atomic reads with the native
TAT command: the product's region list and one exact Lighthouse agent status.
Both require an explicit request region, owner and verified caller. The fixed
endpoint/version is independent of Lighthouse compute availability. No hidden
credential renewal, page loop, task dispatch or guest operation follows.

Regions require checked unique IDs and matching native count; agent reads send
one lhins ID with Offset=0/Limit=1 and reject foreign/duplicate/count conflicts.
Missing/null collections or counts refuse; an empty checked agent set remains
unlisted. Optional missing/null facts stay unknown, explicit feature [] stays
empty, unknown finite labels are withheld and mark schema coverage incomplete.
Coverage describes the known schema, not freshness or enrollment eligibility.
Agent heartbeat is checked RFC 3339 evidence; consumers own freshness policy.
Raw provider prose, unknown fields and region display names are withheld.
Hardrig owns eligibility, durable enrollment, public-key binding, task policy,
SSH trust and recovery; this observation capability grants none of them.

CVM, network and TAT receipt paths alias one canonical generic Tencent Receipt;
the public fields and serialization remain unchanged. Identity is housed in its
existing namespace directory without changing public paths or custody behavior.

Feishu chat get reads minimum metadata for one explicit chat through the fixed
official chats.get endpoint and open_id selector. SDK chat::Get/Chat and
Client::chat match im chats get. It projects only optional name, mode/type and
external status; a missing field stays unknown and empty name stays observed
empty. requested binds the caller's exact chat, not a fabricated provider echo:
the official body need not contain chat_id. A present foreign/malformed echo
refuses. Metadata success, absent names and external status prove neither
membership nor historical message access. Consumers own resource choice and
authorization. No name lookup, message read, membership change, pagination,
credential renewal or retry follows. Remaining chat fields and operations are
staged gaps against full lark-cli. Issue #133 owns this subset and its evidence.

TAT additionally dispatches one explicit public UTF-8 SHELL script to one
Lighthouse through RunCommand and reads its scoped DescribeInvocationTasks result.
Run fixes SaveCommand=false and EnableParameter=false, with explicit username,
absolute remote Linux directory and timeout; no parameters, COS or secret payload.
Accepted binds returned command/invocation/instance and input SHA-256, not success.
RunCommand has no ClientToken: consumers persist exact intent before dispatch;
unknown writes cannot be replayed or recovered by name/time-window adoption.

Task lookup uses one exact invocation/instance filter or one exact task ID, then
checks invocation, command, instance and optional task ID in the returned row.
Checked empty remains unlisted; nullable facts remain unknown. Reads retain known
finite statuses, times, signed exit, dropped count and command metadata/digest.
Output is explicitly opt-in, decoded bytes in Rust and canonical base64 in JSON;
hidden output stays withheld even if returned. Raw command content, provider
error prose and external output URLs are withheld. Unknown fields/labels and
external COS output or nonempty upload error information mark coverage incomplete.
The documented upload error field is withheld; missing, null or empty is inert.
Schema coverage is no success,
freshness, absence, guest privilege or SSH trust proof. No hidden read, waiter,
page loop, refresh, retry or compensation is introduced. Hardrig owns orchestration,
script/output policy, durable receipts, enrollment and trust registration.

`runseal::tencent::lighthouse` and `Client::lighthouse` share finite read-only
SDK/CLI operations: exact DescribeInstances, exact DescribeKeyPairs and one
explicit key page. Native lhins/lhkp IDs, region and verified caller remain
mandatory. Each operation sends one API without refresh, retry, pagination,
mutation or trust registration. Generic `Lookup<T>` owns exact ID/value/count/
coverage observations; checked empty means unlisted, never authoritative absence.

Instance observations retain UUID/create time, native zone/platform/state,
separate IPv4/IPv6 addresses, nullable login settings and independent latest
operation/action/state/request/start time. Foreign CVM address/placement fields
refuse. Missing/null facts remain unknown, native [] stays observed empty and
blank operation facts remain unavailable. A documented empty LoginSettings.KeyIds
may mean unavailable evidence; it proves no absence of guest access. Latest
SUCCESS alone cannot settle a different operation or the consumer's intent.

Key observations preserve public material and independent aggregate instance
IDs and instance/user association entries. Multiple users for one instance remain
distinct; neither collection supplies missing facts for the other. Unknown user
and public-key values stay unknown; an empty public string cannot match a key.
PrivateKey, tags and provider error prose never enter DTOs or diagnostics.
Unknown field values/labels and nested association/login coverage propagate
incomplete coverage. Instance schema diagnostics retain at most sixteen public
CamelCase property names, each at most 64 ASCII alphanumeric bytes; other names
stay omitted/count-only. No unknown field value enters these diagnostics, and
every unsupported field still makes coverage incomplete.
The live API additionally returns ActivityLabels, BlueprintImageUrl, DisplayArea
and DisplayLabel, absent from the current public SDK/docs. These four named
extensions have an explicit opaque disposition: retain only their finite field
identity and JSON type, never values, nested contents or URL fetching. Missing
extensions stay missing; null stays observed null. Their business meaning remains
unassessed and grants no ownership, region, enrollment or mutation authority.
Finite projection coverage accounts for these opaque observations; it does not
claim their semantics or full provider business-schema coverage.
Coverage concerns this finite enrollment projection and known top-level
fields, not a full storage/billing model, guest privilege or safe mutation proof.
Consumers own cross-page coverage, freshness, ownership and enrollment policy.

The common Client is housed with identity custody and CAM wire models/tests with
their command adapter. Public Client/type paths, credential-state version,
existing CLI grammar and public TAT contracts retain their behavior. No Plumb
or Locus dependency enters the reusable library. This read surface grants no
key import/binding, lifecycle authority, SSH trust or actual management access.

Lighthouse additionally imports one named public RSA key, deletes one exact key,
and associates one exact key/instance/explicit Linux user through fixed ONLINE.
Import shares the CVM bounded 2048..4096-bit RSA validator and returns the native
ID plus SHA256 fingerprint of submitted material; that fingerprint binds intent,
not a provider observation. No private material, tags or generated keys enter
these operations. CLI reads a bounded public file without printing its contents.
Mutation receipts are accepted, never observed convergence. Unexpected response
fields preserve incomplete coverage; malformed acknowledgements stay unknown.
No client token, hidden read, refresh, retry, fallback to OFFLINE or compensation
exists. Persist exact intent before dispatch; never replay unknown writes.
ONLINE avoids shutdown, but does not promise preservation of existing credentials.
Hardrig owns guest-impact review, ownership, latest-operation correlation,
per-user binding observation, SSH authentication and independent host trust.
Delete does not detach keys; callers qualify current references and own recovery.

## CAM Policy Retirement

`runseal::tencent::cam` exposes plain public Request/Data/Entity/Page models and
`Client::cam`, reusing the existing CAM capability group. Native `cam policy
entities --id ID --page N` and `cam policy delete --id ID` consume that same
contract. Both require an explicit owning account and verified current caller;
other CAM commands retain their existing contracts. Native private custody and
the session's held Permit remain shared, without a CLI subprocess bridge.

Entities sends one ListEntitiesForPolicy page with EntityFilter=All, Rp=100 and
an explicit positive page. It retains the requested policy reference, page,
limit, total, next and finite entity ID/relation/UIN projection. This reference
binds the request, not a fabricated provider echo. User/group/role relations stay
distinct, zero UIN stays observed zero, and missing/null UIN remains unknown.
Names and attachment-time values are withheld; their supported opaque shapes
contribute coverage. Unknown fields or relations mark coverage incomplete.
Missing/null counts/arrays, malformed identifiers, count conflicts and duplicate
relation/ID pairs refuse. Complete means finite page projection coverage, not
exhaustive pagination, ownership, an attachment lock or permission to delete.

Delete sends exactly one ID in the provider's PolicyId array. It performs no
pre-read, detach, page loop, refresh, retry or compensation. Accepted means an
acknowledgement, never converged absence. Unknown transport or malformed results
retain uncertain outcome and cannot authorize replay. Fixed mainland global CAM
requests omit X-TC-Region; the generic receipt's region is the empty string.
Safe documented policy-ID refusal codes remain diagnostics, not absence proof.

Consumers persist exact intent and independently qualify original ownership,
unchanged policy metadata/document and fresh complete association coverage.
They own pagination, race policy, exact postconditions and recovery. Empty pages
or a single not-found diagnostic prove no global absence or effective-permission
revocation. SDK fixtures establish implementation behavior, not real account
authority or retirement of a particular policy. Hardrig #197's four temporary
definitions are the separate installed RC/stable qualification consumer in
Runseal #168; standing operator/observer/DNS grants and the production instance
remain outside its deletion set.
