# Runseal v0.17.0

Runseal now includes an atomic `@cloudflare` HTTP dialect. It covers the full
account-owned and user-owned API token lifecycle, permission discovery, Worker
service and domain perception, and R2 bucket and custom-domain perception and
teardown.

Token create and roll reserve a new mode-0600 file and write the returned value
there. Ordinary output contains only the path and SHA-256 fingerprint. The
library exposes the value through a redacted, zeroizing secret type.

The shared HTTP layer now preserves status, headers, and non-JSON bodies in a
typed fault. Safe Cloudflare reads retry bounded 429 and server failures while
mutations remain single-attempt.

The selected Cloudflare contracts are checked against the official OpenAPI
repository pinned at commit `4e2f140437b8e356fb28631ece09c26efd7e781c`.
