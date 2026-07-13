import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { env } from "@/lib/std/env.ts";
import { io } from "@/lib/std/io.ts";
import { doc } from "@/lib/std/json.ts";
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
if (flags(args).help()) {
  flags(args).positionals("guard", { allowHelp: true });
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

    const cargo = await bin("cargo").text(["metadata", "--no-deps", "--format-version", "1"]);
    const current = doc(cargo).get(".packages[0].version");
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
    const prior = doc(metadata).has(".guard.version.hash")
      ? doc(metadata).get(".guard.version.hash")
      : "";
    if (prior === "") {
      io.print("guard version policy: stable metadata has no guard.version.hash; skipping");
      return;
    }

    let stable = doc(metadata).has(".stableVersion") ? doc(metadata).get(".stableVersion") : "";
    if (stable === "" && doc(metadata).has(".releaseVersion")) {
      stable = doc(metadata).get(".releaseVersion");
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
await bin("cargo").run(["fmt", "--all", "--check"]);

io.print("==> cargo clippy");
await bin("cargo").run([
  "clippy",
  "--locked",
  "--workspace",
  "--all-targets",
  "--",
  "-D",
  "warnings",
]);

io.print("==> cargo test");
await bin("cargo").run(["test", "--locked", "--workspace"]);

io.print("==> deno fmt");
await bin("deno").run(["fmt", "--check", ".runseal"]);

io.print("==> deno check");
await bin("deno").run([
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
await bin("negentropy").run(["--strict", "."]);

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
  await bin(command).run(["-n", script]);
}
