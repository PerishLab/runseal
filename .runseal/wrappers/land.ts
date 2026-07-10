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
  base: string;
  body: string;
  repo: string;
  dryRun: boolean;
  deleteBranch: boolean;
};

function usage(): void {
  io.print("Usage: runseal :land [options]");
  io.print("");
  io.print("Land the current clean topic branch on Forgejo.");
  io.print("The branch is pushed, a PR is created or reused, guard is awaited,");
  io.print("the PR is squash-merged, main is synced, and the topic branch is deleted.");
  io.print("");
  io.print("Options:");
  io.print("  --base <branch>    base branch (default: main)");
  io.print("  --body <body>      pull request body override");
  io.print("  --repo <owner/name> Forgejo repository (default: derived from origin)");
  io.print("  --dry-run          print planned actions without changing git or Forgejo");
  io.print("  --no-delete        keep the topic branch after merge");
}

function parse(args: string[]): Options & { help: boolean } {
  const parsed = parseCliArgs(args, {
    string: ["base", "body", "repo"],
    boolean: ["dry-run", "no-delete", "help", "h"],
  });
  requireNoPositionals(parsed, "land", { allowHelp: true });
  return {
    base: stringOption(parsed, "base", "main"),
    body: stringOption(parsed, "body"),
    repo: stringOption(parsed, "repo"),
    dryRun: booleanOption(parsed, "dry-run"),
    deleteBranch: !booleanOption(parsed, "no-delete"),
    help: helpRequested(parsed),
  };
}

const options = parse([...Deno.args]);
if (options.help) {
  usage();
  Deno.exit(0);
}

await cmd.run("git", ["--version"], { stdout: "null" });

const branch = await current();
const repo = options.repo === "" ? await target() : options.repo;
if (options.dryRun) {
  await landable(options.base, branch, { fetch: false });
  plan(options, repo, branch);
  Deno.exit(0);
}

await landable(options.base, branch, { fetch: true });
await cmd.run("git", ["push", "-u", "origin", branch]);

const pr = await pull(options, repo, branch);
const number = json.get(pr, ".number");
const url = json.get(pr, ".html_url");
io.print(url);
const sha = await guarded(repo, number);
await merge(repo, number, sha, options.deleteBranch);
await cmd.run("git", ["checkout", options.base]);
await cmd.run("git", ["pull", "--ff-only", "origin", options.base]);
if (options.deleteBranch && await ok(["rev-parse", "--verify", `refs/heads/${branch}`])) {
  await cmd.run("git", ["branch", "-D", branch]);
}

async function current(): Promise<string> {
  const branch = await cmd.text("git", ["branch", "--show-current"]);
  if (branch === "") {
    io.fail("land: detached HEAD is not a landable topic branch");
  }
  return branch;
}

async function landable(
  base: string,
  branch: string,
  options: { fetch: boolean },
): Promise<void> {
  if (branch === base || branch === "main" || branch === "master") {
    io.fail(`land: must run on a topic branch, not ${branch}`);
  }
  const dirty = await cmd.text("git", ["status", "--short"]);
  if (dirty.trim() !== "") {
    io.fail("land: working tree must be clean; commit or discard changes first");
  }
  if (options.fetch) {
    await cmd.run("git", ["fetch", "origin", base]);
  }
  const remote = `origin/${base}`;
  if (!await ok(["rev-parse", "--verify", remote])) {
    io.fail(`land: missing ${remote}; fetch or check the base branch name`);
  }
  if (!await ok(["merge-base", "--is-ancestor", remote, "HEAD"])) {
    io.fail(`land: current branch must contain latest ${remote}; rebase onto ${base} first`);
  }
  const ahead = Number(await cmd.text("git", ["rev-list", "--count", `${remote}..HEAD`]));
  if (!Number.isFinite(ahead) || ahead <= 0) {
    io.fail(`land: current branch has no commits ahead of ${remote}`);
  }
}

async function ok(args: string[]): Promise<boolean> {
  return await cmd.status("git", args, {
    stdin: "null",
    stdout: "null",
    stderr: "null",
  }) === 0;
}

async function pull(options: Options, repo: string, branch: string): Promise<string> {
  const existing = await runseal.text([
    "@tool",
    "forgejo",
    "pr",
    "find",
    "--repo",
    repo,
    "--head",
    branch,
    "--base",
    options.base,
  ]);
  if (!json.empty(existing)) {
    return existing;
  }

  const args = [
    "@tool",
    "forgejo",
    "pr",
    "create",
    "--repo",
    repo,
    "--base",
    options.base,
    "--head",
    branch,
    "--title",
    await title(options.base),
  ];
  if (options.body !== "") {
    args.push("--body", options.body);
  }
  return await runseal.text(args);
}

async function title(base: string): Promise<string> {
  const subjects = await cmd.text("git", [
    "log",
    "--reverse",
    "--format=%s",
    `origin/${base}..HEAD`,
  ]);
  const first = subjects.split(/\r?\n/).find((line) => line.trim() !== "");
  return first ?? "land branch";
}

async function guarded(repo: string, number: string): Promise<string> {
  const run = await runseal.text([
    "@tool",
    "forgejo",
    "pr",
    "guard",
    "--repo",
    repo,
    "--number",
    number,
  ]);
  return json.get(run, ".commit_sha");
}

async function merge(repo: string, number: string, sha: string, remove: boolean): Promise<void> {
  await runseal.run([
    "@tool",
    "forgejo",
    "pr",
    "merge",
    "--repo",
    repo,
    "--number",
    number,
    "--head",
    sha,
    "--delete-branch",
    String(remove),
  ]);
}

async function target(): Promise<string> {
  const origin = (await cmd.text("git", ["remote", "get-url", "origin"])).replace(/\.git$/, "");
  const found = origin.match(/[:/]([^/:]+)\/([^/]+)$/);
  if (found === null) {
    return io.fail(`land: cannot derive Forgejo owner/name from origin: ${origin}`);
  }
  return `${found[1]}/${found[2]}`;
}

function plan(options: Options, repo: string, branch: string): void {
  const creation = options.body === "" ? "--title <commit>" : "--title <commit> --body <given>";
  const steps = [
    "[dry-run] would run:",
    `  git fetch origin ${options.base}`,
    `  verify ${branch} is clean, not ${options.base}, contains origin/${options.base}, ahead >= 1`,
    `  git push -u origin ${branch}`,
    `  runseal @tool forgejo pr find --repo ${repo} --head ${branch} --base ${options.base}`,
    `  runseal @tool forgejo pr create --repo ${repo} --base ${options.base} --head ${branch} ${creation}  # if missing`,
    `  runseal @tool forgejo pr guard --repo ${repo} --number <n>`,
    `  runseal @tool forgejo pr merge --repo ${repo} --number <n> --head <guarded-sha> --delete-branch ${options.deleteBranch}`,
    `  git checkout ${options.base}`,
    `  git pull --ff-only origin ${options.base}`,
  ];
  if (options.deleteBranch) {
    steps.push(`  git branch -D ${branch}  # if still present locally`);
  }
  io.print(steps.join("\n"));
}
