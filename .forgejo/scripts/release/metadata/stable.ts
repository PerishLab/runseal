const USER_AGENT = "runseal-release-stable/1.0";
const BOOTSTRAP_404_RETRY_MS = 15000;
const STABLE = /^(\d+)\.(\d+)\.(\d+)$/;
const TAGGED_STABLE = /^v?(\d+\.\d+\.\d+)$/;

function fail(message: string): never {
  console.error(`[release-stable] ${message}`);
  Deno.exit(1);
}

function tuple(value: string): [number, number, number] {
  const match = STABLE.exec(value);
  if (!match) {
    fail(`expected stable x.y.z version, got ${value}`);
  }
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function order(left: string, right: string): number {
  const a = tuple(left);
  const b = tuple(right);
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) {
      return a[i] < b[i] ? -1 : 1;
    }
  }
  return 0;
}

async function cargoVersion(): Promise<string> {
  const text = await Deno.readTextFile("app/Cargo.toml");
  const match = /^version = "([^"]+)"$/m.exec(text);
  if (!match) {
    fail("missing version in Cargo.toml");
  }
  tuple(match[1]);
  return match[1];
}

function parseStable(value: string, source: string): string {
  const match = TAGGED_STABLE.exec(value);
  if (!match) {
    fail(`${source} must look like vX.Y.Z, got ${value}`);
  }
  return match[1];
}

async function output(name: string, value: string): Promise<void> {
  const path = Deno.env.get("GITHUB_OUTPUT");
  if (path) {
    await Deno.writeTextFile(path, `${name}=${value}\n`, { append: true });
  }
}

async function tryFetch(url: string): Promise<[string | null, number | null]> {
  try {
    const response = await fetch(url, {
      headers: { "Cache-Control": "no-cache", "User-Agent": USER_AGENT },
    });
    if (response.ok) {
      return [await response.text(), null];
    }
    await response.body?.cancel();
    return [null, response.status];
  } catch (error) {
    fail(`failed to fetch R2 stable metadata: ${error}`);
  }
}

async function fetchOptional(url: string): Promise<string | null> {
  let [text, code] = await tryFetch(url);
  if (text !== null) {
    return text;
  }
  if (code === 403) {
    fail("R2 stable metadata returned HTTP 403; permission errors must not be treated as missing");
  }
  if (code === 404) {
    console.log(
      `[release-stable] R2 stable metadata 404; retrying after ${BOOTSTRAP_404_RETRY_MS}ms`,
    );
    await new Promise((resolve) => setTimeout(resolve, BOOTSTRAP_404_RETRY_MS));
    [text, code] = await tryFetch(url);
    if (text !== null) {
      return text;
    }
    if (code === 403) {
      fail(
        "R2 stable metadata returned HTTP 403 on retry; refusing to bootstrap on permission error",
      );
    }
    if (code === 404) {
      return null;
    }
  }
  fail(`failed to fetch R2 stable metadata: HTTP ${code}`);
}

function readMetadataStable(metadata: Record<string, unknown>): string {
  const value = metadata.stableVersion || metadata.releaseVersion;
  if (typeof value === "string" && value) {
    return parseStable(value, "R2 stable metadata");
  }
  const base = metadata.baseVersion;
  if (typeof base === "string") {
    tuple(base);
    return base;
  }
  fail("R2 stable metadata must include stableVersion, releaseVersion, or baseVersion");
}

async function nextStable(cargo: string): Promise<[string, string, string]> {
  const publicUrl = (Deno.env.get("RUNSEAL_RELEASES_PUBLIC_URL") ?? "").replace(/\/+$/, "");
  let metadataUrl = Deno.env.get("RUNSEAL_STABLE_METADATA_URL");
  if (!metadataUrl) {
    if (!publicUrl) {
      fail("RUNSEAL_RELEASES_PUBLIC_URL is required");
    }
    metadataUrl = `${publicUrl}/stable/latest/metadata.json`;
  }
  console.log(`[release-stable] metadata url: ${metadataUrl}`);
  const text = await fetchOptional(metadataUrl);
  if (text === null) {
    console.log(`[release-stable] no R2 stable metadata; releasing first stable v${cargo}`);
    return [cargo, `v${cargo}`, "missing R2 stable metadata"];
  }
  let metadata: unknown;
  try {
    metadata = JSON.parse(text);
  } catch (error) {
    fail(`R2 stable metadata is invalid JSON: ${error}`);
  }
  if (typeof metadata !== "object" || metadata === null) {
    fail("R2 stable metadata must be a JSON object");
  }
  const prior = readMetadataStable(metadata as Record<string, unknown>);
  const ranked = order(cargo, prior);
  if (ranked < 0) {
    fail(`Cargo version ${cargo} regressed below prior stable ${prior}`);
  }
  if (ranked === 0) {
    fail(`Cargo version ${cargo} matches the prior stable; bump Cargo.toml before re-running`);
  }
  return [cargo, `v${cargo}`, `R2 stable metadata v${prior}`];
}

async function main(): Promise<void> {
  const cargo = await cargoVersion();
  const override = (Deno.env.get("STABLE_VERSION_OVERRIDE") ?? "").trim();
  let base: string;
  let release: string;
  let source: string;
  if (override) {
    base = parseStable(override, "STABLE_VERSION_OVERRIDE");
    if (base !== cargo) {
      fail(`override base ${base} does not match Cargo version ${cargo}`);
    }
    release = `v${base}`;
    source = "workflow override";
  } else {
    [base, release, source] = await nextStable(cargo);
  }
  console.log("[release-stable] channel: stable");
  console.log(`[release-stable] base version: ${base}`);
  console.log(`[release-stable] release version: ${release}`);
  console.log(`[release-stable] state source: ${source}`);
  await output("base_version", base);
  await output("release_version", release);
  await output("state_source", source);
}

await main();
