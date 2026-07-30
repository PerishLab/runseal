import { cache } from "@perish/sealkit/cache";
import { cli, flags } from "@perish/sealkit/cli";
import { bin } from "@perish/sealkit/cmd";
import { io } from "@perish/sealkit/io";

function usage(): void {
  io.print("Usage: runseal :guard [--fresh]");
  io.print("");
  io.print("Run the complete repository guard.");
  io.print("");
  io.print("Options:");
  io.print("  --fresh          ignore the guard cache and run the full gauntlet");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h", "fresh"] });
flags(args).positionals("guard", { allowHelp: true });
if (flags(args).help()) {
  usage();
  Deno.exit(0);
}

io.print("==> plumb doctor");
await bin("plumb").run(["doctor", "."]);

const mark = await cache.key([["ectropy", ["--version"]]]);
if (args.fresh !== true && (await cache.hit(mark))) {
  io.print(`guard: clean (cached ${mark.slice(0, 12)})`);
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

io.print("==> cargo check release");
await bin("cargo").run([
  "check",
  "--locked",
  "--workspace",
  "--all-targets",
  "--release",
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
]);

io.print("==> ectropy");
await bin("ectropy").run(["."]);

await cache.keep(mark);
