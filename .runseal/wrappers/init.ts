import { cli, flags } from "@perish/harness/cli";
import { init } from "@perish/harness/init";
import { io } from "@perish/harness/io";

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
flags(args).positionals("init", { allowHelp: true });
if (flags(args).help()) {
  io.print("Usage: runseal :init");
  io.print("");
  io.print("Validate the repository and install versioned git hooks.");
  Deno.exit(0);
}

await init({
  tools: [
    "git",
    "deno",
    "cargo",
    "runseal",
    "sh",
    "bash",
    "sed",
    "grep",
  ],
  paths: [
    "Cargo.toml",
    "Cargo.lock",
    "negentropy.toml",
    "vocabulary.toml",
    "docs/vocabulary.md",
    "manage.sh",
    "runseal.toml",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/negentropy.version",
    ".runseal/hooks/pre-commit",
    ".runseal/hooks/commit-msg",
    ".runseal/templates/cloudflare.env",
    ".runseal/wrappers/cloudflare.ts",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".runseal/wrappers/release.ts",
    ".forgejo/release.env.example",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-beta.yml",
    ".forgejo/workflows/release-stable.yml",
    ".forgejo/scripts/release/assets/checksums.sh",
    ".forgejo/scripts/release/assets/package.sh",
    ".forgejo/scripts/release/assets/verify.sh",
    ".forgejo/scripts/release/metadata/beta.ts",
    ".forgejo/scripts/release/metadata/stable.ts",
    ".forgejo/scripts/release/r2/check.sh",
    ".forgejo/scripts/release/r2/publish.sh",
    ".forgejo/scripts/release/r2/summary.sh",
    ".forgejo/scripts/release/r2/verify.sh",
    ".forgejo/scripts/release/smoke/smoke.sh",
  ],
});
