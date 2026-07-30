# Cloudflare Tool Examples

These examples show the intended `runseal @tool cloudflare ...` atom shapes and
how repo wrappers can bind local policy around them.

## Credential file

Cloudflare helpers read repo-local credentials from:

```text
.local/secrets/cloudflare.env
```

Create the template with:

```bash
runseal :cloudflare init
```

The template is local operator state. Do not commit filled credential files.

## Config

Inspect the configured defaults that wrappers will use:

```bash
runseal @tool cloudflare config json
runseal @tool cloudflare config get zone_name
```

## Generic API Request

Use `api request` for a single authenticated Cloudflare API request when no
more specific atom exists:

```bash
runseal @tool cloudflare api request GET /zones --query name=perish.uk
```

Send a JSON request body with `--json`:

```bash
runseal @tool cloudflare api request PATCH /zones/ZONE_ID/dns_records/RECORD_ID \
  --json '{"ttl":120}'
```

## Zone And DNS Records

Fetch one zone by exact name:

```bash
runseal @tool cloudflare zone get --name perish.uk
```

List DNS records:

```bash
runseal @tool cloudflare zone dns-record list --zone-id ZONE_ID
runseal @tool cloudflare zone dns-record list --zone-id ZONE_ID --name runseal.perish.uk
```

Create or update one DNS record from explicit JSON:

```bash
runseal @tool cloudflare zone dns-record create --zone-id ZONE_ID \
  --json '{"type":"CNAME","name":"runseal","content":"example.com","proxied":true}'

runseal @tool cloudflare zone dns-record update --zone-id ZONE_ID --record-id RECORD_ID \
  --json '{"ttl":120}'
```

## Rulesets

List and fetch rulesets:

```bash
runseal @tool cloudflare zone ruleset list --zone-id ZONE_ID
runseal @tool cloudflare zone ruleset get --zone-id ZONE_ID --ruleset-id RULESET_ID
```

Add or update a rule with the ruleset atoms:

```bash
runseal @tool cloudflare zone ruleset rule add \
  --zone-id ZONE_ID \
  --ruleset-id RULESET_ID \
  --json '{"action":"redirect"}'

runseal @tool cloudflare zone ruleset rule update \
  --zone-id ZONE_ID \
  --ruleset-id RULESET_ID \
  --rule-id RULE_ID \
  --json '{"description":"updated"}'
```

## Wrapper Boundary

Use `runseal :cloudflare ...` for repository-owned credential initialization,
service checks, and authenticated API calls:

```bash
runseal :cloudflare init
runseal :cloudflare check
runseal :cloudflare api GET /zones --query name=perish.uk
```

Use direct `@tool cloudflare ...` calls for one atomic API operation. Use the
wrapper when the flow needs repo defaults, repeated calls, JSON shaping, or
operator policy.
