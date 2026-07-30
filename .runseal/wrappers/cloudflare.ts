import { cli, flags } from "@perish/sealkit/cli";
import { Cloudflare, keys } from "@perish/sealkit/cloudflare";
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
  };
}

function seated(value: unknown): Record<string, unknown> {
  if (typeof value === "object" && value !== null && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
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
    io.print(`zone: ${cfg.zone} (${id})`);
    io.print(`zone rulesets: ${rulesets.length}`);
    io.print("zones:");
    io.print(pretty(zones));
    io.print("r2 buckets:");
    io.print(pretty(buckets));
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
  case "api":
    await new Op(rest).api();
    break;
  default:
    io.fail(`cloudflare: unknown command: ${command}`);
}
