import {
  booleanOption,
  helpRequested,
  parseArgs as parseCliArgs,
  requireNoPositionals,
  stringOption,
} from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { runseal } from "@/lib/std/runseal.ts";

type Options = {
  channel: string;
  repo: string;
  ref: string;
  version: string;
  watch: boolean;
  dryRun: boolean;
};

function workflow(channel: string): string {
  switch (channel) {
    case "stable":
      return "release-stable.yml";
    case "beta":
      return "release-beta.yml";
    default:
      return io.fail(`invalid choice: ${channel}`, 2);
  }
}

function usage(): void {
  io.print("Usage: runseal :release --channel=stable|beta [options]");
  io.print("");
  io.print("Trigger one Forgejo release workflow for the selected channel.");
  io.print("Use --ref for branch beta runs; the default ref is main.");
  io.print("");
  io.print("Options:");
  io.print("  --channel <name>      release channel: stable or beta");
  io.print("  --repo <owner/name>   Forgejo repository (default: derived from origin)");
  io.print("  --ref <ref>           git ref passed to the workflow (default: main)");
  io.print("  --version <version>   optional release version override, e.g. v0.9.0-beta.2");
  io.print("  --watch              watch the triggered workflow run");
  io.print("  --dry-run            print planned action without triggering a workflow");
}

function parse(args: string[]): Options & { help: boolean; argc: number } {
  const parsed = parseCliArgs(args, {
    string: ["channel", "repo", "ref", "version"],
    boolean: ["watch", "dry-run", "help", "h"],
  });
  requireNoPositionals(parsed, "release", { allowHelp: true });
  return {
    channel: stringOption(parsed, "channel"),
    repo: stringOption(parsed, "repo"),
    ref: stringOption(parsed, "ref", "main"),
    version: stringOption(parsed, "version"),
    watch: booleanOption(parsed, "watch"),
    dryRun: booleanOption(parsed, "dry-run"),
    help: helpRequested(parsed),
    argc: args.length,
  };
}

const options = parse([...Deno.args]);
if (options.argc === 0 || options.help) {
  usage();
  Deno.exit(0);
}
if (options.channel === "") {
  io.fail("release: --channel is required");
}

const file = workflow(options.channel);
const repo = options.repo === "" ? await target() : options.repo;

const dry =
  `runseal @tool forgejo workflow dispatch --repo ${repo} --workflow ${file} --ref ${options.ref} --input version_override=${options.version}`;
if (options.dryRun) {
  io.print(dry);
  Deno.exit(0);
}

const raw = await runseal.text([
  "@tool",
  "forgejo",
  "workflow",
  "dispatch",
  "--repo",
  repo,
  "--workflow",
  file,
  "--ref",
  options.ref,
  "--input",
  `version_override=${options.version}`,
]);
const id = json.get(raw, ".id");
io.print(`triggered ${file} run ${id} for ref ${options.ref}`);

if (options.watch) {
  await runseal.run([
    "@tool",
    "forgejo",
    "run",
    "watch",
    "--repo",
    repo,
    "--id",
    id,
    "--interval",
    "10",
  ]);
}

async function target(): Promise<string> {
  const origin = (await cmd.text("git", ["remote", "get-url", "origin"])).replace(/\.git$/, "");
  const found = origin.match(/[:/]([^/:]+)\/([^/]+)$/);
  if (found === null) {
    return io.fail(`release: cannot derive Forgejo owner/name from origin: ${origin}`);
  }
  return `${found[1]}/${found[2]}`;
}
