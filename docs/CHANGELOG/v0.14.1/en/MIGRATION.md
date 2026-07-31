# Migrating to Runseal v0.14.1

This release replaces the wrapper host with the profile closure.

Classify every old wrapper before deleting it:

- invoke a plumbable external CLI through `runseal :[profile] <command>`;
- move generic guard, init, land, and release behavior to the substrate or its
  canonical workflow;
- return product workflow to the product CLI that owns it;
- introduce `@tool` only for one atomic third-party operation that cannot be
  isolated by the profile triad.

Do not translate wrapper files into another scripting language or into Runseal
command lists.

Rewrite `runseal.toml` as profile data. Old `resources`, `deno`, and
`injections` sections are not accepted:

```toml
[env]
unset = ["AMBIENT_KEY"]

[env.vars]
TOKEN_FILE = "local://secrets/token"

[argv]
cargo = ["--locked"]

[[symlink]]
source = "resource://tool.conf"
target = "local://run/tool.conf"
```

Move inert committed files to `.runseal/resources`, keep secrets under
`.local`, and remove `.runseal/deno.json`, `.runseal/deno.lock`, generic
wrappers, and repository-owned Git hooks once their behavior has an owner.

Use `runseal : <command>` for the default profile and
`runseal :<name> <command>` for a named profile. No `--` separator is needed.
An unmarked token is always an internal Runseal command.
