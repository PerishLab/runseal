const USER_AGENT = "runseal-release-beta/1.0";
const BOOTSTRAP_404_RETRY_MS = 15000;
const STABLE = /^(\d+)\.(\d+)\.(\d+)$/;
const CARGO_BETA = /^(\d+\.\d+\.\d+)-beta\.(?:0|[1-9][0-9]*)$/;
const BETA = /^v?(\d+\.\d+\.\d+)-beta\.([1-9][0-9]*)$/;

function fail(message: string): never {
  console.error(`[release-beta] ${message}`);
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
  const beta = CARGO_BETA.exec(match[1]);
  if (beta) {
    return beta[1];
  }
  tuple(match[1]);
  return match[1];
}

function parseBeta(value: string, source: string): [string, number, string] {
  const match = BETA.exec(value);
  if (!match) {
    fail(`${source} must look like vX.Y.Z-beta.N, got ${value}`);
  }
  const base = match[1];
  const number = Number(match[2]);
  return [base, number, `v${base}-beta.${number}`];
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
    fail(`failed to fetch R2 beta metadata: ${error}`);
  }
}

async function fetchOptional(url: string): Promise<string | null> {
  let [text, code] = await tryFetch(url);
  if (text !== null) {
    return text;
  }
  if (code === 403) {
    fail("R2 beta metadata returned HTTP 403; permission errors must not be treated as missing");
  }
  if (code === 404) {
    console.log(`[release-beta] R2 beta metadata 404; retrying after ${BOOTSTRAP_404_RETRY_MS}ms`);
    await new Promise((resolve) => setTimeout(resolve, BOOTSTRAP_404_RETRY_MS));
    [text, code] = await tryFetch(url);
    if (text !== null) {
      return text;
    }
    if (code === 403) {
      fail(
        "R2 beta metadata returned HTTP 403 on retry; refusing to bootstrap on permission error",
      );
    }
    if (code === 404) {
      return null;
    }
  }
  fail(`failed to fetch R2 beta metadata: HTTP ${code}`);
}

function readMetadataBeta(metadata: Record<string, unknown>): [string, number, string] {
  const value = metadata.betaVersion || metadata.releaseVersion;
  if (typeof value === "string" && value) {
    return parseBeta(value, "R2 beta metadata");
  }
  const base = metadata.baseVersion;
  const number = metadata.betaNumber;
  if (typeof base === "string" && typeof number === "number") {
    tuple(base);
    if (number < 1) {
      fail(`R2 beta metadata betaNumber must be >= 1, got ${number}`);
    }
    return [base, number, `v${base}-beta.${number}`];
  }
  fail("R2 beta metadata must include betaVersion or releaseVersion");
}

async function nextBeta(cargo: string): Promise<[string, number, string, string]> {
  const publicUrl = (Deno.env.get("RUNSEAL_RELEASES_PUBLIC_URL") ?? "").replace(/\/+$/, "");
  let metadataUrl = Deno.env.get("RUNSEAL_BETA_METADATA_URL");
  if (!metadataUrl) {
    if (!publicUrl) {
      fail("RUNSEAL_RELEASES_PUBLIC_URL is required");
    }
    metadataUrl = `${publicUrl}/beta/latest/metadata.json`;
  }
  console.log(`[release-beta] metadata url: ${metadataUrl}`);
  const text = await fetchOptional(metadataUrl);
  if (text === null) {
    console.log("[release-beta] no R2 beta metadata; starting beta.1");
    return [cargo, 1, `v${cargo}-beta.1`, "missing R2 beta metadata"];
  }
  let metadata: unknown;
  try {
    metadata = JSON.parse(text);
  } catch (error) {
    fail(`R2 beta metadata is invalid JSON: ${error}`);
  }
  if (typeof metadata !== "object" || metadata === null) {
    fail("R2 beta metadata must be a JSON object");
  }
  const [base, number, betaVersion] = readMetadataBeta(metadata as Record<string, unknown>);
  const ranked = order(cargo, base);
  if (ranked < 0) {
    fail(`Cargo version ${cargo} regressed below beta base ${base}`);
  }
  if (ranked > 0) {
    return [cargo, 1, `v${cargo}-beta.1`, "R2 beta metadata base advanced"];
  }
  return [cargo, number + 1, `v${cargo}-beta.${number + 1}`, `R2 beta metadata ${betaVersion}`];
}

async function main(): Promise<void> {
  const cargo = await cargoVersion();
  const override = (Deno.env.get("BETA_VERSION_OVERRIDE") ?? "").trim();
  let base: string;
  let number: number;
  let version: string;
  let source: string;
  if (override) {
    [base, number, version] = parseBeta(override, "BETA_VERSION_OVERRIDE");
    if (base !== cargo) {
      fail(`override base ${base} does not match Cargo version ${cargo}`);
    }
    source = "workflow override";
  } else {
    [base, number, version, source] = await nextBeta(cargo);
  }
  console.log("[release-beta] channel: beta");
  console.log(`[release-beta] base version: ${base}`);
  console.log(`[release-beta] beta number: ${number}`);
  console.log(`[release-beta] beta version: ${version}`);
  console.log(`[release-beta] state source: ${source}`);
  await output("base_version", base);
  await output("beta_number", String(number));
  await output("beta_version", version);
  await output("release_version", version);
  await output("state_source", source);
}

await main();
