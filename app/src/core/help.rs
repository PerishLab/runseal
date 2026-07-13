use anyhow::{Result, bail};

pub fn resolve(name: &str, args: &[String]) -> Result<Option<&'static str>> {
    if !matches!(args, [arg] if matches!(arg.as_str(), "-h" | "--help" | "help")) {
        return Ok(None);
    }
    text(name).map(Some)
}

fn text(name: &str) -> Result<&'static str> {
    match name {
        "profile" => Ok(PROFILE),
        "resolve" => Ok(RESOLVE),
        "resources" => Ok(RESOURCES),
        "tool" => Ok(crate::core::tool::help()),
        "wrappers" => Ok(WRAPPERS),
        "which" => Ok(WHICH),
        _ => bail!("unknown internal command: @{name}"),
    }
}

const PROFILE: &str = "\
Usage: runseal @profile

Print the resolved runseal runtime paths for the current invocation.

Output:
  RUNSEAL_HOME          runseal configuration root
  RUNSEAL_PROFILE_HOME  default profile directory
  RUNSEAL_PROFILE_PATH  selected profile file
  RUNSEAL_RESOURCE_ROOT resolved resource root, when configured
  RUNSEAL_WRAPPER_PATH  wrapper search path

Profile discovery:
  1. --profile <path>
  2. runseal.toml|yaml|yml|json from the current directory upward
  3. $RUNSEAL_PROFILE_HOME/default.toml|yaml|yml|json

@profile is read-only and does not run profile injections.
";

const RESOURCES: &str = "\
Usage: runseal @resources

Print RUNSEAL_RESOURCE_ROOT for the selected profile.

A profile must declare:

  [resources]
  root = \".local\"

The root may be relative to the profile directory, absolute, or ~ expanded.
@resources is read-only and does not create, import, export, or validate files.
";

const RESOLVE: &str = "\
Usage: runseal @resolve resource://... [resource://...]

Resolve one or more resource:// paths against the selected profile's resource root.

Examples:
  runseal @resolve resource://
  runseal @resolve resource://ssh/config resource://kube

resource:// values are profile-only path literals. Child commands receive resolved
absolute paths from env injection values; scripts do not need to understand resource://.

Invalid resource paths include empty segments, '.', '..', backslashes, and ':' inside
path segments. Resolved paths are printed even when the target file does not exist.
";

const WRAPPERS: &str = "\
Usage: runseal @wrappers

List the effective wrappers visible to the selected profile.

Lookup order:
  1. <profile-dir>/.runseal/wrappers/<name>.ts
  2. <profile-dir>/.runseal/wrappers/<name>.sh
  3. $RUNSEAL_HOME/wrappers/<name>.ts
  4. $RUNSEAL_HOME/wrappers/<name>.sh

Profile-local wrappers shadow home wrappers with the same name. On Unix, wrapper
shell files use the .sh suffix and must be executable. Deno wrappers use the .ts
suffix and are executed with deno using the selected profile's [deno] policy.
Extensionless files in a wrappers directory are not wrapper entrypoints; migrate
legacy wrappers to <name>.ts or <name>.sh. On Windows, runseal also checks .exe,
.cmd, and .bat when the wrapper name has no extension.

Use .ts wrappers for structured cross-platform operations and keep reusable
domain atoms in @tool. Use .sh for thin Unix bootstrap glue.

@wrappers is read-only and does not run profile injections.
";

const WHICH: &str = "\
Usage: runseal @which :<wrapper>

Print the absolute path for the wrapper selected by runseal's wrapper lookup.

Examples:
  runseal @which :ssh
  runseal @which :release

@which currently supports only :wrapper arguments. It is read-only and does not run
profile injections.
";
