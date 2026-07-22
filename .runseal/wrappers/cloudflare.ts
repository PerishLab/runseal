import { cli, flags } from "@perish/sealkit/cli";
import { Cloudflare, exact, keys } from "@perish/sealkit/cloudflare";
import { env } from "@perish/sealkit/env";
import { fs } from "@perish/sealkit/fs";
import { io } from "@perish/sealkit/io";

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
  io.print("  api <method> <path>       authenticated Cloudflare API call [--query k=v ...]");
  io.print("");
  io.print("Credentials:");
  io.print("  .local/secrets/cloudflare.env");
}

function reject(value: string | undefined, message: string): void {
  if (value !== undefined && value !== "") {
    io.fail(message);
  }
}

function pretty(value: unknown): string {
  return JSON.stringify(value, null, 2) ?? "null";
}

type Forge = {
  api: Cloudflare;
  account: string;
  zone: string;
  host: string;
  origin: string;
  prefix: string;
};

async function forge(): Promise<Forge> {
  const held = await keys();
  const token = held.CLOUDFLARE_API_TOKEN ?? "";
  const account = held.CLOUDFLARE_ACCOUNT_ID ?? "";
  if (token === "" || account === "") {
    io.fail("cloudflare: fill CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID first");
  }
  const base = env.get("CLOUDFLARE_API_BASE", "https://api.cloudflare.com/client/v4");
  return {
    api: new Cloudflare(token, base),
    account,
    zone: held.CLOUDFLARE_ZONE_NAME || "perish.uk",
    host: held.CLOUDFLARE_MANAGE_HOST || "runseal.perish.uk",
    origin: held.CLOUDFLARE_MANAGE_ORIGIN_HOST || "releases.runseal.perish.uk",
    prefix: held.CLOUDFLARE_MANAGE_REDIRECT_PREFIX ?? "",
  };
}

const mark = "runseal_manage_sh_redirect";
const phase = "http_request_dynamic_redirect";

function seated(value: unknown): Record<string, unknown> {
  if (typeof value === "object" && value !== null && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

class Manage {
  constructor(private readonly cfg: Forge) {}

  plotted(): Record<string, unknown> {
    const target = this.cfg.prefix === ""
      ? `https://${this.cfg.origin}/manage.sh`
      : `https://${this.cfg.origin}/${this.cfg.prefix}/manage.sh`;
    return exact({
      reference: mark,
      description: "Redirect runseal manage.sh to releases bucket asset",
      host: this.cfg.host,
      path: "/manage.sh",
      url: target,
    });
  }

  show(rule: Record<string, unknown>, id?: string): void {
    io.print("manage redirect plan");
    io.print(`zone: ${this.cfg.zone}`);
    if (id !== undefined) {
      io.print(`zone id: ${id}`);
    }
    io.print(`request host: ${this.cfg.host}`);
    io.print(`redirect host: ${this.cfg.origin}`);
    io.print(`phase: ${phase}`);
    io.print("rules:");
    io.print(pretty(rule));
  }

  async resolve(zone: string): Promise<Record<string, unknown>> {
    const listed = await this.cfg.api.rulesets(zone);
    const found = listed.map(seated).find((entry) => entry.phase === phase);
    if (found === undefined) {
      return seated(
        await this.cfg.api.phase(zone, {
          kind: "zone",
          name: "Single Redirects ruleset",
          phase,
          rules: [],
        }),
      );
    }
    return seated(await this.cfg.api.ruleset(zone, String(found.id)));
  }

  async upsert(
    slot: { zone: string; ruleset: string },
    current: Record<string, unknown> | undefined,
    payload: Record<string, unknown>,
  ): Promise<string> {
    if (current === undefined) {
      await this.cfg.api.add(slot.zone, slot.ruleset, payload);
      return `created ${mark}`;
    }
    await this.cfg.api.change(slot.zone, slot.ruleset, String(current.id), payload);
    return `updated ${mark}`;
  }
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
    const cfg = await forge();
    const zone = await cfg.api.zone(cfg.zone);
    const id = String(zone.id);
    const rulesets = await cfg.api.rulesets(id);
    const zones = await cfg.api.result("GET", "/zones", {
      "account.id": cfg.account,
      "per_page": "50",
    });
    const account = seated(await cfg.api.result("GET", `/accounts/${cfg.account}`));
    const buckets = await cfg.api.result("GET", `/accounts/${cfg.account}/r2/buckets`);
    io.print("cloudflare check: ok");
    io.print(`account id: ${cfg.account}`);
    io.print(`account name: ${account.name}`);
    io.print(`manage zone: ${cfg.zone} (${id})`);
    io.print(`zone rulesets: ${rulesets.length}`);
    io.print("zones:");
    io.print(pretty(zones));
    io.print("r2 buckets:");
    io.print(pretty(buckets));
  }

  async plan(): Promise<void> {
    reject(this.rest[0], "cloudflare: manage-plan does not accept arguments");
    const manage = new Manage(await forge());
    manage.show(manage.plotted());
  }

  async inspect(): Promise<void> {
    reject(this.rest[0], "cloudflare: manage-inspect does not accept arguments");
    const cfg = await forge();
    const zone = await cfg.api.zone(cfg.zone);
    const id = String(zone.id);
    const listed = await cfg.api.rulesets(id);
    const found = listed.map(seated).find((entry) => entry.phase === phase);
    if (found === undefined) {
      io.print(`manage inspect: no ${phase} zone ruleset found`);
      return;
    }
    const full = seated(await cfg.api.ruleset(id, String(found.id)));
    const rules = Array.isArray(full.rules) ? full.rules.map(seated) : [];
    const matched = rules.filter((rule) => rule.ref === mark);
    io.print(`zone id: ${id}`);
    io.print(`ruleset id: ${full.id}`);
    io.print(`ruleset name: ${full.name}`);
    if (matched.length === 0) {
      io.print("manage inspect: no manage redirect rules found");
      return;
    }
    io.print("manage rules:");
    io.print(pretty(matched));
  }

  async ensure(): Promise<void> {
    const args = cli.parse(this.rest, {
      boolean: ["dry-run"],
      unknownOptionMessage: (arg) => `cloudflare: unknown manage-ensure-redirect argument: ${arg}`,
    });
    flags(args).positionals("cloudflare: manage-ensure-redirect");
    const dry = flags(args).boolean("dry-run");
    const cfg = await forge();
    const manage = new Manage(cfg);
    const rule = manage.plotted();
    const zone = await cfg.api.zone(cfg.zone);
    const id = String(zone.id);
    if (dry) {
      manage.show(rule, id);
      return;
    }
    const ruleset = await manage.resolve(id);
    const rid = String(ruleset.id);
    const rules = Array.isArray(ruleset.rules) ? ruleset.rules.map(seated) : [];
    const current = rules.find((entry) => entry.ref === mark);
    const change = await manage.upsert({ zone: id, ruleset: rid }, current, rule);
    io.print("manage ensure redirect: ok");
    io.print(`  - ${change}`);
  }

  async api(): Promise<void> {
    const [method, path, ...extra] = this.rest;
    if (method === undefined || path === undefined) {
      io.fail("cloudflare: api requires a method and a path");
    }
    const query: Record<string, string> = {};
    for (let at = 0; at < extra.length; at += 1) {
      if (extra[at] !== "--query") {
        io.fail(`cloudflare: unknown api argument: ${extra[at]}`);
      }
      const pair = extra[at + 1] ?? "";
      const split = pair.indexOf("=");
      if (split < 0) {
        io.fail("cloudflare: --query expects key=value");
      }
      query[pair.slice(0, split)] = pair.slice(split + 1);
      at += 1;
    }
    const cfg = await forge();
    io.print(JSON.stringify(await cfg.api.request(method, path, query)));
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
