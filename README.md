# runseal

Run a command inside a small, explicit profile.

Use `runseal` when a repository needs explicit local resources, named wrappers,
and env/argv/symlink setup without becoming a task runner, secret manager, or
deployment orchestrator.

It is designed for repos that carry private local paths such as `.local`,
kubeconfig files, SSH config, tool directories, or wrapper scripts, and need
one command to expose those paths to child tools predictably.

## 30-second Example

Declare profile-local resources:

```toml
[resources]
root = ".local"

[[injections]]
type = "env"

[injections.vars]
APP_SSH_CONFIG = "resource://ssh/config"
APP_SECRET_DIR = "resource://secrets"
```

Inspect what runseal resolved:

```bash
runseal @profile
runseal @resources
runseal @resolve resource://ssh/config
```

Run an external command or a named wrapper inside the profile:

```bash
runseal bash -lc 'echo "$APP_SSH_CONFIG"'
runseal :deploy --dry-run
```

## Inspect What Runseal Sees

Runseal inspection commands are read-only and do not run profile injections:

```bash
runseal @profile
runseal @resources
runseal @resolve resource:// resource://ssh/config
runseal @wrappers
runseal @which :deploy
```

These commands answer the first debugging questions: which profile was selected,
where resources resolve, which wrappers are visible, and which wrapper file a
`:name` command will execute.

## Command Routing

Command routing is based on the first command token:

- `runseal <cmd>` runs an external command inside the profile.
- `runseal :<cmd>` runs a profile wrapper.
- `runseal @<cmd>` runs a runseal-owned command.

For example:

```bash
runseal --profile ./runseal.toml bash -lc 'echo "$RUNSEAL_PROFILE_PATH"'
```

Use `runseal profile` without `@` to run an external command named `profile`.

## Examples

Repository-owned examples for canonical `@tool` and wrapper boundary shapes
live under [docs/examples](./docs/examples/README.md).

Start here when an operator flow needs the exact runseal form:

- [GitHub tool examples](./docs/examples/tools/github.md)

## Fit

Fits well:

- repo-local private resources
- explicit one-command execution environments
- named wrappers with discoverability
- passing profile-scoped paths to tools like `ssh`, `kubectl`, `uv`, or
  `terraform`

Not trying to be:

- a task dependency graph
- a secret lifecycle manager
- a deployment orchestrator
- a shell auto-activation tool

## Capabilities

`runseal` currently supports three profile capabilities plus explicit wrapper
and internal command namespaces:

- `env`: export environment variables and ordered env operations.
- `symlink`: create symlinks for the command lifecycle, then clean them up.
- `argv`: inject fixed arguments after a matching child command token.
- `resource://...`: resolve profile-local resource paths inside env values.
- `[deno]`: declare the repo-level Deno wrapper policy for `.ts` wrappers.

## Install

```bash
curl -fsSL https://runseal.perish.uk/manage.sh | sh
```

Install a beta or one explicit version:

```bash
curl -fsSL https://runseal.perish.uk/manage.sh | sh -s -- install --channel beta
curl -fsSL https://runseal.perish.uk/manage.sh | sh -s -- install --version vX.Y.Z-beta.N
```

Uninstall:

```bash
curl -fsSL https://runseal.perish.uk/manage.sh | sh -s -- uninstall
```

If `--profile` is omitted, profile discovery walks from the current directory
to filesystem root. At each directory, format priority is:

1. `runseal.toml`
2. `runseal.yaml`
3. `runseal.yml`
4. `runseal.json`

If no ancestor profile is found, discovery falls back to:

1. `$RUNSEAL_PROFILE_HOME/default.toml`
2. `$RUNSEAL_PROFILE_HOME/default.yaml`
3. `$RUNSEAL_PROFILE_HOME/default.yml`
4. `$RUNSEAL_PROFILE_HOME/default.json`

`RUNSEAL_HOME` is the runseal configuration root. When unset it defaults to `~/.runseal`.

`RUNSEAL_PROFILE_HOME` is the profile directory. When unset it defaults to `$RUNSEAL_HOME/profiles`.

Each child command receives:

- `RUNSEAL_HOME`
- `RUNSEAL_PROFILE_HOME`
- `RUNSEAL_PROFILE_PATH`
- `RUNSEAL_WRAPPER_PATH`

## Profile

```toml
[resources]
root = ".local"

[[injections]]
type = "env"

[injections.vars]
RUNSEAL_ENV = "dev"
LOCAL_ROOT = "resource://"
SSH_CONFIG = "resource://ssh/config"

[[injections]]
type = "env"

[[injections.ops]]
op = "prepend"
key = "PATH"
value = "./bin"
separator = "os"
dedup = true

[[injections]]
type = "symlink"
source = "./tool"
target = "./.runseal-bin/tool"
on_exist = "replace"
cleanup = true

[[injections]]
type = "argv"
command = "ssh"
args = ["-F", ".local/ssh/config"]
```

Lifecycle symlink targets are single-owner. Do not run concurrent `runseal`
invocations that manage the same `target` with `cleanup = true`; one process can
replace or remove the link while another still expects to own it. Use distinct
targets when commands need to run in parallel under the same profile.

`resource://path/to/file` is a profile-only path literal. A profile that uses
resource URIs must declare:

```toml
[resources]
root = ".local"
```

The resource root may be relative to the profile directory, absolute, or `~`
expanded. In env injection values, runseal rewrites resource URIs to absolute
paths under that configured root. For example, with `root = ".local"`,
`resource://ssh/config` resolves to `<profile-dir>/.local/ssh/config`.
`resource://` and `resource://.` resolve to the resource root itself.

Child commands receive only the resolved absolute path. They do not receive
or need to understand `resource://`.

Resource URIs are resolved only when the env value is exactly the URI. runseal
does not perform partial string interpolation inside env values.

Resource paths must be relative URI-style paths. Empty paths, empty path
segments, `.`, `..`, backslash separators, and `:` inside path segments are
rejected. Resource paths are resolved without checking whether the file exists.

## Wrappers

If the command token starts with `:`, runseal resolves it as a wrapper
executable instead of a literal program name:

```bash
runseal :deploy --dry-run
```

Wrapper lookup order is:

1. `<profile-dir>/.runseal/wrappers/<name>.ts`
2. `<profile-dir>/.runseal/wrappers/<name>.sh`
3. `$RUNSEAL_HOME/wrappers/<name>.ts`
4. `$RUNSEAL_HOME/wrappers/<name>.sh`

The profile directory is the directory containing `RUNSEAL_PROFILE_PATH`.
Successful profile and wrapper paths are normalized absolute paths.
The child working directory is not changed. A resolved wrapper receives:

- `RUNSEAL_WRAPPER_NAME`
- `RUNSEAL_WRAPPER_FILE`

Deno wrappers use the `.ts` suffix and are executed with `deno run` using the
selected profile's `[deno]` policy. On Unix, shell wrappers use the `.sh`
suffix and must be executable. Extensionless files in `.runseal/wrappers` are
not wrapper entrypoints; migrate legacy wrappers to `<name>.ts` or `<name>.sh`.
On Windows, runseal also checks `.exe`, `.cmd`, and `.bat` when the wrapper
name has no extension.

### Deno wrappers

Use `.ts` wrappers for structured cross-platform operations: argument parsing,
JSON handling, HTTP, polling, and small control flow that would otherwise turn
into an uncontrolled Python or shell dependency stack. Deno fits runseal when it
is an explicit single-binary prerequisite with a repo-declared permission
policy.

Declare the policy in the selected profile:

```toml
[deno]
config = ".runseal/deno.json"
lock = ".runseal/deno.lock"
permissions = [
  "--allow-read=.",
  "--allow-write=.",
  "--allow-env",
  "--allow-net",
  "--allow-run=git,gh,runseal",
]
```

Permission entries expand host environment references such as
`--allow-read=${HOME}/.tea/tea.yml` before Deno starts. A referenced variable
must exist.

Runseal adds `--no-prompt`, then the configured `--config`, `--lock`
`--frozen=true`, and permission flags before the wrapper file and caller args.
The wrapper still receives `RUNSEAL_WRAPPER_NAME`, `RUNSEAL_WRAPPER_FILE`,
`RUNSEAL_PROFILE_PATH`, `RUNSEAL_HOME`, `RUNSEAL_PROFILE_HOME`, and
`RUNSEAL_WRAPPER_PATH`.

Wrapper helper modules live in the repo under `.runseal/lib/`. Import them
directly through the repo import map; do not treat generic helpers as public
Rust `@tool` namespaces:

```ts
import { json } from "@/lib/std/json.ts";
import { runseal } from "@/lib/std/runseal.ts";

const zone = await runseal.text([
  "@tool",
  "cloudflare",
  "zone",
  "get",
  "--name",
  "perish.uk",
]);
console.log(json.get(zone, ".id"));
```

Keep reusable domain atoms in `@tool` only when they remain product-domain
operations that are not better expressed in Deno wrapper code. The current
built-in tool surface is intentionally small: GitHub helpers and Cloudflare
helpers. Use wrappers to bind repo-local policy, defaults, and flow around those
atoms.

Prefer visible repo or local artifacts under `.runseal/` or `.local/` for
multi-line config and payload text. The wrapper should usually validate
preconditions, choose files, assemble arguments, and invoke the operational
flow, rather than build large inline text blobs.

## Internal Commands

If the command token starts with `@`, runseal resolves it as a runseal internal
command instead of a literal program name:

```bash
runseal @profile
runseal @resources
runseal @resolve resource:// resource://ssh/config
runseal @wrappers
runseal @which :deploy
```

Runseal-owned commands do not run profile injections. Inspection commands are
read-only; `@tool` is the explicit atomic tool runtime.

- `@profile` prints the resolved runseal runtime paths. If resources are
  configured, it also prints `RUNSEAL_RESOURCE_ROOT`.
- `@resources` prints the resolved resource root.
- `@resolve resource://...` prints resolved absolute resource paths, one per
  argument.
- `@tool <namespace> <command> ...` runs an atomic runseal tool command. The
  current public namespaces are `forgejo`, `github`, and `cloudflare`. Run
  `runseal @tool --help` for the complete tool index. Tools are reusable atoms:
  they may read generic
  defaults such as service tokens, but profile-specific paths and env names
  should be supplied by the calling wrapper.
- `@wrappers` lists the effective wrappers visible to the current profile.
- `@which :<name>` prints the wrapper file that `:<name>` resolves to.

YAML and JSON profiles use the same structure:

```yaml
resources:
  root: .local
injections:
  - type: env
    vars:
      LOCAL_ROOT: resource://
      SSH_CONFIG: resource://ssh/config
```

```json
{
  "resources": {
    "root": ".local"
  },
  "injections": [
    {
      "type": "env",
      "vars": {
        "LOCAL_ROOT": "resource://",
        "SSH_CONFIG": "resource://ssh/config"
      }
    }
  ]
}
```

## Validation

Initialize local development hooks:

```bash
runseal :init
```

Commit-driven validation is the default path:

```bash
git commit
```

The generated pre-commit hook runs `runseal :guard`. For explicit manual
validation before committing or while diagnosing a failure:

```bash
runseal :guard
```
