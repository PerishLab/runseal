import { cli } from "@/lib/cli.ts";
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

function reject(value: string | undefined, message: string): void {
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

async function load(): Promise<ManageRules> {
  const zone = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const request = await runseal.text(["@tool", "cloudflare", "config", "get", "manage_host"]);
  const redirect = await runseal.text([
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
  const target = prefix === ""
    ? `https://${redirect}/manage.sh`
    : `https://${redirect}/${prefix}/manage.sh`;
  const rule = await runseal.text([
    "@tool",
    "cloudflare",
    "redirect-rule",
    "exact",
    "--ref",
    "runseal_manage_sh_redirect",
    "--description",
    "Redirect runseal manage.sh to releases bucket asset",
    "--host",
    request,
    "--path",
    "/manage.sh",
    "--target-url",
    target,
  ]);
  return { zoneName: zone, requestHost: request, redirectHost: redirect, ruleSh: rule };
}

async function print(rules: ManageRules, id?: string): Promise<void> {
  const pretty = json.pretty(rules.ruleSh);
  io.print("manage redirect plan");
  io.print(`zone: ${rules.zoneName}`);
  if (id !== undefined) {
    io.print(`zone id: ${id}`);
  }
  io.print(`request host: ${rules.requestHost}`);
  io.print(`redirect host: ${rules.redirectHost}`);
  io.print("phase: http_request_dynamic_redirect");
  io.print("rules:");
  io.print(pretty);
}

async function init(rest: string[]): Promise<void> {
  reject(rest[0], "cloudflare: init does not accept arguments");
  const paths = {
    local: env.get("RUNSEAL_REPO_LOCAL_DIR", ".local"),
    secrets: env.get("RUNSEAL_REPO_SECRETS_DIR", ".local/secrets"),
    tmp: env.get("RUNSEAL_REPO_TMP_DIR", ".local/tmp"),
  };
  const token = `${paths.secrets}/cloudflare.env`;
  await fs.dir.ensure(paths.local, "700");
  await fs.dir.ensure(paths.secrets, "700");
  await fs.dir.ensure(paths.tmp, "700");
  if (await fs.file.exists(token)) {
    io.print(`exists ${token}`);
    return;
  }
  const template = await Deno.readTextFile(".runseal/templates/cloudflare.env");
  await fs.file.writeText(token, template, "600");
  await fs.file.chmodIfUnix(token, "600");
  io.print(`created ${token}`);
}

async function check(rest: string[]): Promise<void> {
  reject(rest[0], "cloudflare: check does not accept arguments");
  const config = {
    account: await runseal.text(["@tool", "cloudflare", "config", "get", "account_id"]),
    zone: await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]),
  };
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", config.zone]);
  const id = json.get(zone, ".id");
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    id,
  ]);
  const response = await runseal.text([
    "@tool",
    "cloudflare",
    "api",
    "request",
    "GET",
    "/zones",
    "--query",
    `account.id=${config.account}`,
    "--query",
    "per_page=50",
  ]);
  const zones = json.get(response, ".result");
  const account = await runseal.text([
    "@tool",
    "cloudflare",
    "account",
    "get",
    "--account-id",
    config.account,
  ]);
  const buckets = await runseal.text([
    "@tool",
    "cloudflare",
    "account",
    "r2",
    "bucket",
    "list",
    "--account-id",
    config.account,
  ]);
  io.print("cloudflare check: ok");
  io.print(`account id: ${config.account}`);
  io.print(`account name: ${json.get(account, ".name")}`);
  io.print(`manage zone: ${config.zone} (${id})`);
  io.print(`zone rulesets: ${json.len(rulesets)}`);
  io.print("zones:");
  io.print(json.pretty(zones));
  io.print("r2 buckets:");
  io.print(json.pretty(buckets));
}

async function plan(rest: string[]): Promise<void> {
  reject(rest[0], "cloudflare: manage-plan does not accept arguments");
  await print(await load());
}

async function inspect(rest: string[]): Promise<void> {
  reject(rest[0], "cloudflare: manage-inspect does not accept arguments");
  const name = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
  const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", name]);
  const id = json.get(zone, ".id");
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    id,
  ]);
  const ruleset = json.find(rulesets, "phase", "http_request_dynamic_redirect");
  if (ruleset === "") {
    io.print("manage inspect: no http_request_dynamic_redirect zone ruleset found");
    return;
  }
  const rid = json.get(ruleset, ".id");
  const full = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "get",
    "--zone-id",
    id,
    "--ruleset-id",
    rid,
  ]);
  const rules = json.get(full, ".rules");
  const matched = json.filter(rules, "ref", ["runseal_manage_sh_redirect"]);
  io.print(`zone id: ${id}`);
  io.print(`ruleset id: ${rid}`);
  io.print(`ruleset name: ${json.get(full, ".name")}`);
  if (json.len(matched) === 0) {
    io.print("manage inspect: no manage redirect rules found");
    return;
  }
  const pretty = json.pretty(matched);
  io.print("manage rules:");
  io.print(pretty);
}

async function resolve(zone: string): Promise<string> {
  const rulesets = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "list",
    "--zone-id",
    zone,
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
      zone,
      "--phase",
      "http_request_dynamic_redirect",
      "--name",
      "Single Redirects ruleset",
    ]);
  }
  const id = json.get(ruleset, ".id");
  ruleset = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "get",
    "--zone-id",
    zone,
    "--ruleset-id",
    id,
  ]);
  return ruleset;
}

async function upsert(
  zone: string,
  ruleset: string,
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
      zone,
      "--ruleset-id",
      ruleset,
      "--json",
      payload,
    ]);
    return `created ${ref}`;
  }
  const id = json.get(current, ".id");
  await runseal.run([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "rule",
    "update",
    "--zone-id",
    zone,
    "--ruleset-id",
    ruleset,
    "--rule-id",
    id,
    "--json",
    payload,
  ]);
  return `updated ${ref}`;
}

async function ensure(rest: string[]): Promise<void> {
  const args = cli.parse(rest, {
    boolean: ["dry-run"],
    unknownOptionMessage: (arg) => `cloudflare: unknown manage-ensure-redirect argument: ${arg}`,
  });
  cli.positionals(args, "cloudflare: manage-ensure-redirect");
  const dry = cli.boolean(args, "dry-run");
  const rules = await load();
  const zone = await runseal.text([
    "@tool",
    "cloudflare",
    "zone",
    "get",
    "--name",
    rules.zoneName,
  ]);
  const id = json.get(zone, ".id");
  if (dry) {
    await print(rules, id);
    return;
  }
  const ruleset = await resolve(id);
  const rid = json.get(ruleset, ".id");
  const current = json.get(ruleset, ".rules");
  const change = await upsert(
    id,
    rid,
    json.find(current, "ref", "runseal_manage_sh_redirect"),
    "runseal_manage_sh_redirect",
    rules.ruleSh,
  );
  io.print("manage ensure redirect: ok");
  io.print(`  - ${change}`);
}

async function api(rest: string[]): Promise<void> {
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
    await init(rest);
    break;
  case "check":
    await check(rest);
    break;
  case "manage-plan":
    await plan(rest);
    break;
  case "manage-inspect":
    await inspect(rest);
    break;
  case "manage-ensure-redirect":
    await ensure(rest);
    break;
  case "api":
    await api(rest);
    break;
  default:
    io.fail(`cloudflare: unknown command: ${command}`);
}
