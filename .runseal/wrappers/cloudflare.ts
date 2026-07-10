import { booleanOption, parseArgs, requireNoPositionals } from "@/lib/cli.ts";
import { env } from "@/lib/std/env.ts";
import { fs } from "@/lib/std/fs.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";
import { runseal } from "@/lib/std/runseal.ts";

function usage(): void {
  io.print("Usage: runseal :cloudflare <command> [args]");
  io.print("");
  io.print("Commands:");
  io.print("  init                      create repo-local .local/secrets/cloudflare.env template");
  io.print(
    "  check                     validate repo-local credentials and probe core account APIs",
  );
  io.print("  manage-plan               print the desired manage redirect rule shape");
  io.print("  manage-inspect            inspect current dynamic redirect ruleset for manage rules");
  io.print(
    "  manage-ensure-redirect    create/update exact-path manage redirects (use --dry-run first)",
  );
  io.print(
    "  api                       use: runseal @tool cloudflare api request <method> <path> ...",
  );
  io.print("");
  io.print("Credentials:");
  io.print("  .local/secrets/cloudflare.env");
}

function rejectExtraArg(value: string | undefined, message: string): void {
  if (value !== undefined && value !== "") {
    io.fail(message);
  }
}

type ManageRules = {
  zoneName: string;
  requestHost: string;
  redirectHost: string;
  ruleSh: string;
};

async function loadManageRedirectRules(): Promise<ManageRules> {
  const zoneName = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const requestHost = await runseal.text(["@tool", "cloudflare", "config", "get", "manage_host"]);
  const redirectHost = await runseal.text([
    "@tool",
    "cloudflare",
    "config",
    "get",
    "manage_origin_host",
  ]);
  const prefix = await runseal.text([
    "@tool",
    "cloudflare",
    "config",
    "get",
    "manage_redirect_prefix",
  ]);
  const targetSh = prefix === ""
    ? `https://${redirectHost}/manage.sh`
    : `https://${redirectHost}/${prefix}/manage.sh`;
  const ruleSh = await runseal.text([
    "@tool",
    "cloudflare",
    "redirect-rule",
    "exact",
    "--ref",
    "runseal_manage_sh_redirect",
    "--description",
    "Redirect runseal manage.sh to releases bucket asset",
    "--host",
    requestHost,
    "--path",
    "/manage.sh",
    "--target-url",
    targetSh,
  ]);
  return { zoneName, requestHost, redirectHost, ruleSh };
}

async function printManageRedirectPlan(rules: ManageRules, zoneId?: string): Promise<void> {
  const prettySh = json.pretty(rules.ruleSh);
  io.print("manage redirect plan");
  io.print(`zone: ${rules.zoneName}`);
  if (zoneId !== undefined) {
    io.print(`zone id: ${zoneId}`);
  }
  io.print(`request host: ${rules.requestHost}`);
  io.print(`redirect host: ${rules.redirectHost}`);
  io.print("phase: http_request_dynamic_redirect");
  io.print("rules:");
  io.print(prettySh);
}

async function initCommand(rest: string[]): Promise<void> {
  rejectExtraArg(rest[0], "cloudflare: init does not accept arguments");
  const localDir = env.get("RUNSEAL_REPO_LOCAL_DIR", ".local");
  const secretsDir = env.get("RUNSEAL_REPO_SECRETS_DIR", ".local/secrets");
  const tmpDir = env.get("RUNSEAL_REPO_TMP_DIR", ".local/tmp");
  const tokenFile = `${secretsDir}/cloudflare.env`;
  await fs.dir.ensure(localDir, "700");
  await fs.dir.ensure(secretsDir, "700");
  await fs.dir.ensure(tmpDir, "700");
  if (await fs.file.exists(tokenFile)) {
    io.print(`exists ${tokenFile}`);
    return;
  }
  const template = await Deno.readTextFile(".runseal/templates/cloudflare.env");
  await fs.file.writeText(tokenFile, template, "600");
  await fs.file.chmodIfUnix(tokenFile, "600");
  io.print(`created ${tokenFile}`);
}

async function checkCommand(rest: string[]): Promise<void> {
  rejectExtraArg(rest[0], "cloudflare: check does not accept arguments");
  const accountId = await runseal.text(["@tool", "cloudflare", "config", "get", "account_id"]);
  const zoneName = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", zoneName]);
  const zoneId = json.get(zone, ".id");
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    zoneId,
  ]);
  const rulesetCount = json.len(rulesets);
  const zonesPayload = await runseal.text([
    "@tool",
    "cloudflare",
    "api",
    "request",
    "GET",
    "/zones",
    "--query",
    `account.id=${accountId}`,
    "--query",
    "per_page=50",
  ]);
  const zones = json.get(zonesPayload, ".result");
  const zonesPretty = json.pretty(zones);
  const account = await runseal.text([
    "@tool",
    "cloudflare",
    "account",
    "get",
    "--account-id",
    accountId,
  ]);
  const accountName = json.get(account, ".name");
  const buckets = await runseal.text([
    "@tool",
    "cloudflare",
    "account",
    "r2",
    "bucket",
    "list",
    "--account-id",
    accountId,
  ]);
  const bucketsPretty = json.pretty(buckets);
  io.print("cloudflare check: ok");
  io.print(`account id: ${accountId}`);
  io.print(`account name: ${accountName}`);
  io.print(`manage zone: ${zoneName} (${zoneId})`);
  io.print(`zone rulesets: ${rulesetCount}`);
  io.print("zones:");
  io.print(zonesPretty);
  io.print("r2 buckets:");
  io.print(bucketsPretty);
}

async function managePlanCommand(rest: string[]): Promise<void> {
  rejectExtraArg(rest[0], "cloudflare: manage-plan does not accept arguments");
  await printManageRedirectPlan(await loadManageRedirectRules());
}

async function manageInspectCommand(rest: string[]): Promise<void> {
  rejectExtraArg(rest[0], "cloudflare: manage-inspect does not accept arguments");
  const zoneName = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", zoneName]);
  const zoneId = json.get(zone, ".id");
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    zoneId,
  ]);
  const ruleset = json.find(rulesets, "phase", "http_request_dynamic_redirect");
  if (ruleset === "") {
    io.print("manage inspect: no http_request_dynamic_redirect zone ruleset found");
    return;
  }
  const rulesetId = json.get(ruleset, ".id");
  const fullRuleset = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "get",
    "--zone-id",
    zoneId,
    "--ruleset-id",
    rulesetId,
  ]);
  const rulesetName = json.get(fullRuleset, ".name");
  const rules = json.get(fullRuleset, ".rules");
  const matched = json.filter(rules, "ref", ["runseal_manage_sh_redirect"]);
  const matchedCount = json.len(matched);
  io.print(`zone id: ${zoneId}`);
  io.print(`ruleset id: ${rulesetId}`);
  io.print(`ruleset name: ${rulesetName}`);
  if (matchedCount === 0) {
    io.print("manage inspect: no manage redirect rules found");
    return;
  }
  const pretty = json.pretty(matched);
  io.print("manage rules:");
  io.print(pretty);
}

async function resolveManageRedirectRuleset(zoneId: string): Promise<string> {
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    zoneId,
  ]);
  let ruleset = json.find(rulesets, "phase", "http_request_dynamic_redirect");
  if (ruleset === "") {
    return await runseal.text([
      "@tool",
      "cloudflare",
      "zone",
      "ruleset",
      "create",
      "--zone-id",
      zoneId,
      "--phase",
      "http_request_dynamic_redirect",
      "--name",
      "Single Redirects ruleset",
    ]);
  }
  const rulesetId = json.get(ruleset, ".id");
  ruleset = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "get",
    "--zone-id",
    zoneId,
    "--ruleset-id",
    rulesetId,
  ]);
  return ruleset;
}

async function upsertRedirectRule(
  zoneId: string,
  rulesetId: string,
  current: string,
  ref: string,
  payload: string,
): Promise<string> {
  if (current === "") {
    await runseal.run([
      "@tool",
      "cloudflare",
      "zone",
      "ruleset",
      "rule",
      "add",
      "--zone-id",
      zoneId,
      "--ruleset-id",
      rulesetId,
      "--json",
      payload,
    ]);
    return `created ${ref}`;
  }
  const ruleId = json.get(current, ".id");
  await runseal.run([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "rule",
    "update",
    "--zone-id",
    zoneId,
    "--ruleset-id",
    rulesetId,
    "--rule-id",
    ruleId,
    "--json",
    payload,
  ]);
  return `updated ${ref}`;
}

async function manageEnsureRedirectCommand(rest: string[]): Promise<void> {
  const args = parseArgs(rest, {
    boolean: ["dry-run"],
    unknownOptionMessage: (arg) => `cloudflare: unknown manage-ensure-redirect argument: ${arg}`,
  });
  requireNoPositionals(args, "cloudflare: manage-ensure-redirect");
  const dryRun = booleanOption(args, "dry-run");
  const rules = await loadManageRedirectRules();
  const zone = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "get",
    "--name",
    rules.zoneName,
  ]);
  const zoneId = json.get(zone, ".id");
  if (dryRun) {
    await printManageRedirectPlan(rules, zoneId);
    return;
  }
  const ruleset = await resolveManageRedirectRuleset(zoneId);
  const rulesetId = json.get(ruleset, ".id");
  const existingRules = json.get(ruleset, ".rules");
  const changedSh = await upsertRedirectRule(
    zoneId,
    rulesetId,
    json.find(existingRules, "ref", "runseal_manage_sh_redirect"),
    "runseal_manage_sh_redirect",
    rules.ruleSh,
  );
  io.print("manage ensure redirect: ok");
  io.print(`  - ${changedSh}`);
}

async function apiCommand(rest: string[]): Promise<void> {
  if (rest[0] === undefined) {
    io.fail("cloudflare: api requires a method");
  }
  if (rest[1] === undefined) {
    io.fail("cloudflare: api requires a path");
  }
  await runseal.run(["@tool", "cloudflare", "api", "request", ...rest]);
}

const [command, ...rest] = Deno.args;
if (command === undefined || command === "help" || command === "--help") {
  usage();
  Deno.exit(0);
}

switch (command) {
  case "init":
    await initCommand(rest);
    break;
  case "check":
    await checkCommand(rest);
    break;
  case "manage-plan":
    await managePlanCommand(rest);
    break;
  case "manage-inspect":
    await manageInspectCommand(rest);
    break;
  case "manage-ensure-redirect":
    await manageEnsureRedirectCommand(rest);
    break;
  case "api":
    await apiCommand(rest);
    break;
  default:
    io.fail(`cloudflare: unknown command: ${command}`);
}
