# Initial trust primitives

Use Client::open with an explicit owning account, verify identity separately,
and call Client::ram with the exact expected principal and one typed Request.
The client retains its private custody Permit throughout. This subset initially
uses RAMUser bootstrap credentials; routine federated role custody is separate.

Nine operations cover provider get/create, role get/create, custom policy
get/create, exact policy-version get, role-policy list and custom-policy attach.
Provider calls use IMS2019-08-15 at ims.aliyuncs.com; role/policy calls use
RAM2015-05-01 at ram.aliyuncs.com. Native query fields are signed with ACS3 and
RFC3986 encoding, without an official CLI, generic endpoint or action escape.
Creation disables role console login and requires explicit duration. Issuer,
client IDs, CA fingerprints and policy documents are caller-selected inputs.

The SDK performs one provider call. It does not adopt, pre-read, repair, mutate
existing definitions, retire resources, detach, paginate, retry or compensate.
A configuration change requiring those operations is unsupported by this initial
subset and must be planned explicitly. Hardrig owns resource closure and review.

Receipt.requested binds submitted selectors; Data binds observed fields.
Mutation success means accepted, not convergence. Slot preserves missing, null,
zero and empty observations. Complete means supported schema coverage, not live
permissions, exhaustive IAM inventory, eligibility or authority to change it.
DefaultPolicyVersion carries the default policy document; the obsolete
Policy.PolicyDocument field is not used. Unknown fields mark coverage incomplete.
Malformed/foreign responses remain unknown; errors expose no raw provider prose.

IAM resource intents and uncertain outcomes belong to the caller. Persist intent
before a write. Observe the exact target after uncertainty; never replay by
assuming a transport failure means the operation was unsent. Static fixtures
qualify protocol behavior, not real Alibaba permissions or Ensign compatibility.
