import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { fs } from "@/lib/std/fs.ts";
import { io } from "@/lib/std/io.ts";
import { negentropy } from "@/lib/negentropy.ts";
import { path } from "@/lib/std/path.ts";

const HOOKS_PATH = ".runseal/hooks";

function usage(): void {
  io.print("Usage: runseal :init");
  io.print("");
  io.print("Validate the repository and install versioned git hooks.");
}

async function requireTool(name: string): Promise<void> {
  if (!(await cmd.exists(name))) {
    io.fail(`init: missing required tool: ${name}`);
  }
}

async function requirePath(root: string, relPath: string): Promise<void> {
  if (!(await fs.file.exists(path.join(root, relPath)))) {
    io.fail(`init: missing required path: ${relPath}`);
  }
}

const args = parseArgs(Deno.args, { boolean: ["help", "h"] });
requireNoPositionals(args, "init", { allowHelp: true });
if (helpRequested(args)) {
  usage();
  Deno.exit(0);
}

io.print("==> resolving repository");
const root = await cmd.text("git", ["rev-parse", "--show-toplevel"]);
io.print(`repository: ${root}`);

io.print("==> checking required tools");
for (
  const tool of [
    "git",
    "deno",
    "cargo",
    "runseal",
    "sh",
    "bash",
    "sed",
    "grep",
  ]
) {
  await requireTool(tool);
}
await negentropy.verify();
io.print("ok: git, deno, cargo, runseal, negentropy, sh, bash, sed, grep");

io.print("==> checking repository entrypoints");
for (
  const path of [
    "Cargo.toml",
    "Cargo.lock",
    "negentropy.toml",
    "vocabulary.toml",
    "docs/vocabulary.md",
    "manage.sh",
    "runseal.toml",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/negentropy.version",
    ".runseal/hooks/pre-commit",
    ".runseal/hooks/commit-msg",
    ".runseal/lib/cli.ts",
    ".runseal/lib/hash.ts",
    ".runseal/lib/negentropy.ts",
    ".runseal/lib/std/cmd.ts",
    ".runseal/lib/std/env.ts",
    ".runseal/lib/std/fs.ts",
    ".runseal/lib/std/io.ts",
    ".runseal/lib/std/json.ts",
    ".runseal/lib/std/path.ts",
    ".runseal/lib/std/runseal.ts",
    ".runseal/lib/version.ts",
    ".runseal/templates/cloudflare.env",
    ".runseal/wrappers/cloudflare.ts",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".runseal/wrappers/release.ts",
    ".forgejo/release.env.example",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-beta.yml",
    ".forgejo/workflows/release-stable.yml",
    ".forgejo/scripts/release/assets/checksums.sh",
    ".forgejo/scripts/release/assets/package.sh",
    ".forgejo/scripts/release/assets/verify.sh",
    ".forgejo/scripts/release/metadata/beta.ts",
    ".forgejo/scripts/release/metadata/stable.ts",
    ".forgejo/scripts/release/r2/check.sh",
    ".forgejo/scripts/release/r2/publish.sh",
    ".forgejo/scripts/release/r2/summary.sh",
    ".forgejo/scripts/release/r2/verify.sh",
    ".forgejo/scripts/release/smoke/smoke.sh",
  ]
) {
  await requirePath(root, path);
}
io.print("ok: repository entrypoints");

io.print("==> installing git hooks");
await cmd.run("git", ["config", "core.hooksPath", HOOKS_PATH], { cwd: root });
const current = await cmd.text("git", ["config", "--get", "core.hooksPath"], { cwd: root });
io.print(`core.hooksPath = ${current}`);

await cmd.run("deno", ["--version"], { stdout: "null" });
io.print("development environment ready");
