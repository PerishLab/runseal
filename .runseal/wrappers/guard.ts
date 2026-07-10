import { helpRequested, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { env } from "@/lib/std/env.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { treeHash } from "@/lib/hash.ts";
import { negentropy } from "@/lib/negentropy.ts";
import { compareStableVersion, parseStableVersion } from "@/lib/version.ts";

function usage(): void {
  io.print("Usage: runseal :guard [version-check|version-hash]");
  io.print("");
  io.print("Run repository guard checks or one explicit version-policy helper.");
  io.print("");
  io.print("Commands:");
  io.print("  version-check    validate version policy against stable metadata");
  io.print("  version-hash     print the current guard.version.hash value");
}

let mode = "full";
const args = parseArgs(Deno.args, { boolean: ["help", "h"] });
if (helpRequested(args)) {
  requireNoPositionals(args, "guard", { allowHelp: true });
  usage();
  Deno.exit(0);
}
if (args._.length > 0) {
  const arg = args._.shift()!;
  switch (arg) {
    case "version-check":
      mode = "version-check";
      break;
    case "version-hash":
      mode = "version-hash";
      break;
    default:
      io.fail(`guard: unknown command: ${arg}`);
  }
}
if (args._.length > 0) {
  io.fail("guard: unexpected arguments");
}

async function currentHash(): Promise<string> {
  return await treeHash(["app/tests"]);
}

async function versionPolicy(): Promise<void> {
  const publicUrl = env.get("RUNSEAL_RELEASES_PUBLIC_URL", "https://releases.runseal.perish.uk");
  const metadataUrl = env.get(
    "RUNSEAL_STABLE_METADATA_URL",
    `${publicUrl}/stable/latest/metadata.json`,
  );

  const cargoMetadata = await cmd.text("cargo", ["metadata", "--no-deps", "--format-version", "1"]);
  const currentVersion = json.get(cargoMetadata, ".packages[0].version");
  const hash = await currentHash();
  const response = await fetch(`${metadataUrl}?version=${encodeURIComponent(currentVersion)}`);
  if (response.status === 404) {
    io.print("guard version policy: no stable metadata; skipping");
    return;
  }
  if (response.status !== 200) {
    io.fail(`guard version policy: failed to fetch stable metadata: HTTP ${response.status}`);
  }

  const metadata = await response.text();
  const hasPriorHash = json.has(metadata, ".guard.version.hash");
  const priorHash = hasPriorHash ? json.get(metadata, ".guard.version.hash") : "";
  if (priorHash === "") {
    io.print("guard version policy: stable metadata has no guard.version.hash; skipping");
    return;
  }

  const hasStableVersion = json.has(metadata, ".stableVersion");
  let priorVersion = hasStableVersion ? json.get(metadata, ".stableVersion") : "";
  if (priorVersion === "") {
    const hasReleaseVersion = json.has(metadata, ".releaseVersion");
    if (hasReleaseVersion) {
      priorVersion = json.get(metadata, ".releaseVersion");
    }
  }
  if (priorVersion === "") {
    io.fail("guard version policy: stable metadata is missing stableVersion/releaseVersion");
  }

  const currentOrder = compareStableVersion(currentVersion, priorVersion);
  const priorParsed = parseStableVersion(priorVersion);
  const currentParsed = parseStableVersion(currentVersion);
  const sameMinorLineage = currentParsed.major === priorParsed.major &&
    currentParsed.minor === priorParsed.minor;

  if (currentOrder === "lt") {
    io.fail(`guard version policy: version regressed below prior stable ${priorVersion}`);
  }
  if (currentOrder === "eq") {
    io.fail(`guard version policy: version matches prior stable ${priorVersion}`);
  }

  if (hash === priorHash) {
    if (!sameMinorLineage) {
      io.fail(
        `guard version policy: unchanged guard.version.hash requires a patch-only bump above ${priorVersion}`,
      );
    }
    io.print(
      `guard version policy: hash unchanged -> patch bump ok (${priorVersion} -> ${currentVersion})`,
    );
  } else {
    if (sameMinorLineage) {
      io.fail(
        `guard version policy: changed guard.version.hash requires a minor-or-higher bump above ${priorVersion}`,
      );
    }
    io.print(
      `guard version policy: hash changed -> minor-or-higher bump ok (${priorVersion} -> ${currentVersion})`,
    );
  }
}

if (mode === "version-hash") {
  io.print(await currentHash());
  Deno.exit(0);
}

await versionPolicy();
if (mode === "version-check") {
  Deno.exit(0);
}

io.print("==> cargo fmt");
await cmd.run("cargo", ["fmt", "--all", "--check"]);

io.print("==> cargo clippy");
await cmd.run("cargo", [
  "clippy",
  "--locked",
  "--workspace",
  "--all-targets",
  "--",
  "-D",
  "warnings",
]);

io.print("==> cargo test");
await cmd.run("cargo", ["test", "--locked", "--workspace"]);

io.print("==> deno fmt");
await cmd.run("deno", ["fmt", "--check", ".runseal"]);

io.print("==> deno check");
await cmd.run("deno", [
  "check",
  "--config",
  ".runseal/deno.json",
  "--lock",
  ".runseal/deno.lock",
  "--frozen=true",
  ".runseal/wrappers/cloudflare.ts",
  ".runseal/wrappers/guard.ts",
  ".runseal/wrappers/init.ts",
  ".runseal/wrappers/land.ts",
  ".runseal/wrappers/release.ts",
  ".forgejo/scripts/release/metadata/beta.ts",
  ".forgejo/scripts/release/metadata/stable.ts",
]);

io.print("==> negentropy");
await negentropy.verify();
await cmd.run("negentropy", ["--strict", "."]);

io.print("==> shell syntax");
for (
  const [command, script] of [
    ["sh", "manage.sh"],
    ["sh", ".forgejo/scripts/release/assets/checksums.sh"],
    ["sh", ".forgejo/scripts/release/assets/package.sh"],
    ["sh", ".forgejo/scripts/release/assets/verify.sh"],
    ["bash", ".forgejo/scripts/release/r2/check.sh"],
    ["bash", ".forgejo/scripts/release/r2/publish.sh"],
    ["bash", ".forgejo/scripts/release/r2/summary.sh"],
    ["bash", ".forgejo/scripts/release/r2/verify.sh"],
    ["sh", ".forgejo/scripts/release/smoke/smoke.sh"],
  ]
) {
  await cmd.run(command, ["-n", script]);
}
