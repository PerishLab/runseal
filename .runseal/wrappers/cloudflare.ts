import { cli, flags } from "@perish/harness/cli";
import { env } from "@perish/harness/env";
import { fs } from "@perish/harness/fs";
import { io } from "@perish/harness/io";
import { doc } from "@perish/harness/json";
import { runseal } from "@perish/harness/runseal";
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
  const pretty = doc(rules.ruleSh).pretty();
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
  let ruleset = doc(rulesets).find("phase", "http_request_dynamic_redirect");
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
  const id = doc(ruleset).get(".id");
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
type Slot = {
  zone: string;
  ruleset: string;
};
async function upsert(
  slot: Slot,
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
      slot.zone,
      "--ruleset-id",
      slot.ruleset,
      "--json",
      payload,
    ]);
    return `created ${ref}`;
  }
  const id = doc(current).get(".id");
  await runseal.run([
    "@tool",
    "cloudflare",
    "zone",
    "ruleset",
    "rule",
    "update",
    "--zone-id",
    slot.zone,
    "--ruleset-id",
    slot.ruleset,
    "--rule-id",
    id,
    "--json",
    payload,
  ]);
  return `updated ${ref}`;
}
class Op {
  constructor(private readonly rest: string[]) {}

  async init(): Promise<void> {
    reject(this.rest[0], "cloudflare: init does not accept arguments");
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

  async check(): Promise<void> {
    reject(this.rest[0], "cloudflare: check does not accept arguments");
    const config = {
      account: await runseal.text(["@tool", "cloudflare", "config", "get", "account_id"]),
      zone: await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]),
    };
    const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", config.zone]);
    const id = doc(zone).get(".id");
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
    const zones = doc(response).get(".result");
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
    io.print(`account name: ${doc(account).get(".name")}`);
    io.print(`manage zone: ${config.zone} (${id})`);
    io.print(`zone rulesets: ${doc(rulesets).len()}`);
    io.print("zones:");
    io.print(doc(zones).pretty());
    io.print("r2 buckets:");
    io.print(doc(buckets).pretty());
  }

  async plan(): Promise<void> {
    reject(this.rest[0], "cloudflare: manage-plan does not accept arguments");
    await print(await load());
  }

  async inspect(): Promise<void> {
    reject(this.rest[0], "cloudflare: manage-inspect does not accept arguments");
    const name = await runseal.text(["@tool", "cloudflare", "config", "get", "zone_name"]);
    const zone = await runseal.text(["@tool", "cloudflare", "zone", "get", "--name", name]);
    const id = doc(zone).get(".id");
    const rulesets = await runseal.text([
      "@tool",
      "cloudflare",
      "zone",
      "ruleset",
      "list",
      "--zone-id",
      id,
    ]);
    const ruleset = doc(rulesets).find("phase", "http_request_dynamic_redirect");
    if (ruleset === "") {
      io.print("manage inspect: no http_request_dynamic_redirect zone ruleset found");
      return;
    }
    const rid = doc(ruleset).get(".id");
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
    const rules = doc(full).get(".rules");
    const matched = doc(rules).filter("ref", ["runseal_manage_sh_redirect"]);
    io.print(`zone id: ${id}`);
    io.print(`ruleset id: ${rid}`);
    io.print(`ruleset name: ${doc(full).get(".name")}`);
    if (doc(matched).len() === 0) {
      io.print("manage inspect: no manage redirect rules found");
      return;
    }
    const pretty = doc(matched).pretty();
    io.print("manage rules:");
    io.print(pretty);
  }

  async ensure(): Promise<void> {
    const args = cli.parse(this.rest, {
      boolean: ["dry-run"],
      unknownOptionMessage: (arg) => `cloudflare: unknown manage-ensure-redirect argument: ${arg}`,
    });
    flags(args).positionals("cloudflare: manage-ensure-redirect");
    const dry = flags(args).boolean("dry-run");
    const rules = await load();
    const zone = await runseal.text([
      "@tool",
      "cloudflare",
      "zone",
      "get",
      "--name",
      rules.zoneName,
    ]);
    const id = doc(zone).get(".id");
    if (dry) {
      await print(rules, id);
      return;
    }
    const ruleset = await resolve(id);
    const rid = doc(ruleset).get(".id");
    const current = doc(ruleset).get(".rules");
    const change = await upsert(
      { zone: id, ruleset: rid },
      doc(current).find("ref", "runseal_manage_sh_redirect"),
      "runseal_manage_sh_redirect",
      rules.ruleSh,
    );
    io.print("manage ensure redirect: ok");
    io.print(`  - ${change}`);
  }

  async api(): Promise<void> {
    if (this.rest[0] === undefined) {
      io.fail("cloudflare: api requires a method");
    }
    if (this.rest[1] === undefined) {
      io.fail("cloudflare: api requires a path");
    }
    await runseal.run(["@tool", "cloudflare", "api", "request", ...this.rest]);
  }
}

const [command, ...rest] = Deno.args;
if (command === undefined || command === "help" || command === "--help") {
  usage();
  Deno.exit(0);
}
switch (command) {
  case "init":
    await new Op(rest).init();
    break;
  case "check":
    await new Op(rest).check();
    break;
  case "manage-plan":
    await new Op(rest).plan();
    break;
  case "manage-inspect":
    await new Op(rest).inspect();
    break;
  case "manage-ensure-redirect":
    await new Op(rest).ensure();
    break;
  case "api":
    await new Op(rest).api();
    break;
  default:
    io.fail(`cloudflare: unknown command: ${command}`);
}
