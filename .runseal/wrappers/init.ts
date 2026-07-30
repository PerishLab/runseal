import { cli, flags } from "@perish/sealkit/cli";
import { init } from "@perish/sealkit/init";
import { io } from "@perish/sealkit/io";

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
    "ectropy",
    "plumb",
    "runseal",
    "sh",
    "bash",
    "sed",
    "grep",
  ],
  paths: [
    "Cargo.toml",
    "Cargo.lock",
    "ectropy.toml",
    "docs/vocabulary.md",
    "runseal.toml",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/hooks/pre-commit",
    ".runseal/hooks/commit-msg",
    ".runseal/templates/cloudflare.env",
    ".runseal/wrappers/cloudflare.ts",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    "plumb.toml",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-exact.yml",
    ".forgejo/workflows/release-stable.yml",
  ],
});
