import { cli } from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { env } from "@/lib/std/env.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { hash } from "@/lib/hash.ts";
import { negentropy } from "@/lib/negentropy.ts";
import { version } from "@/lib/version.ts";

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
const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (cli.help(args)) {
  cli.positionals(args, "guard", { allowHelp: true });
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

class Policy {
  static async hash(): Promise<string> {
    return await hash.tree(["app/tests"]);
  }

  static async check(): Promise<void> {
    const base = env.get("RUNSEAL_RELEASES_PUBLIC_URL", "https://releases.runseal.perish.uk");
    const url = env.get(
      "RUNSEAL_STABLE_METADATA_URL",
      `${base}/stable/latest/metadata.json`,
    );

    const cargo = await cmd.text("cargo", ["metadata", "--no-deps", "--format-version", "1"]);
    const current = json.get(cargo, ".packages[0].version");
    const digest = await this.hash();
    const response = await fetch(`${url}?version=${encodeURIComponent(current)}`);
    if (response.status === 404) {
      io.print("guard version policy: no stable metadata; skipping");
      return;
    }
    if (response.status !== 200) {
      io.fail(`guard version policy: failed to fetch stable metadata: HTTP ${response.status}`);
    }

    const metadata = await response.text();
    const prior = json.has(metadata, ".guard.version.hash")
      ? json.get(metadata, ".guard.version.hash")
      : "";
    if (prior === "") {
      io.print("guard version policy: stable metadata has no guard.version.hash; skipping");
      return;
    }

    let stable = json.has(metadata, ".stableVersion") ? json.get(metadata, ".stableVersion") : "";
    if (stable === "" && json.has(metadata, ".releaseVersion")) {
      stable = json.get(metadata, ".releaseVersion");
    }
    if (stable === "") {
      io.fail("guard version policy: stable metadata is missing stableVersion/releaseVersion");
    }

    const order = version.compare(current, stable);
    const before = version.parse(stable);
    const after = version.parse(current);
    const lineage = after.major === before.major && after.minor === before.minor;

    if (order === "lt") {
      io.fail(`guard version policy: version regressed below prior stable ${stable}`);
    }
    if (order === "eq") {
      io.fail(`guard version policy: version matches prior stable ${stable}`);
    }

    if (digest === prior) {
      if (!lineage) {
        io.fail(
          `guard version policy: unchanged guard.version.hash requires a patch-only bump above ${stable}`,
        );
      }
      io.print(
        `guard version policy: hash unchanged -> patch bump ok (${stable} -> ${current})`,
      );
    } else {
      if (lineage) {
        io.fail(
          `guard version policy: changed guard.version.hash requires a minor-or-higher bump above ${stable}`,
        );
      }
      io.print(
        `guard version policy: hash changed -> minor-or-higher bump ok (${stable} -> ${current})`,
      );
    }
  }
}

if (mode === "version-hash") {
  io.print(await Policy.hash());
  Deno.exit(0);
}

await Policy.check();
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
