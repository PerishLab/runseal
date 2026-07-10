import { parseArgs as parseStdArgs } from "@std/cli/parse-args";
import type { Args, ParseOptions } from "@std/cli/parse-args";

import { io } from "@/lib/std/io.ts";

type Options = Omit<ParseOptions, "unknown" | "--"> & {
  unknownOptionMessage?: (arg: string) => string;
};

function parse(args: string[], options: Options = {}): Args {
  const { unknownOptionMessage, ...parseOptions } = options;
  validate(args, Array.isArray(parseOptions.string) ? parseOptions.string : []);
  return parseStdArgs(args, {
    "--": true,
    ...parseOptions,
    unknown: (arg) => unknown(arg, unknownOptionMessage),
  });
}

function unknown(arg: string, message?: (arg: string) => string): boolean {
  if (arg.startsWith("-")) {
    io.fail(message?.(arg) ?? `unknown option: ${arg}`);
  }
  return true;
}

function validate(args: string[], names: string[]): void {
  const expected = new Set(names);
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--") {
      return;
    }
    if (!arg.startsWith("--")) {
      continue;
    }
    const [name, value] = arg.slice(2).split("=", 2);
    if (!expected.has(name) || value !== undefined) {
      continue;
    }
    const next = args[index + 1];
    if (next === undefined || next.startsWith("-")) {
      io.fail(`missing value for --${name}`);
    }
  }
}

function help(args: Args): boolean {
  return args.help === true || args.h === true || args._.includes("help");
}

function positionals(
  args: Args,
  context: string,
  options: { allowHelp?: boolean } = {},
): void {
  const extra = args._.find((arg) => !(options.allowHelp === true && arg === "help"));
  if (extra !== undefined) {
    io.fail(`${context}: unexpected argument: ${extra}`);
  }
}

function string(args: Args, name: string, fallback = ""): string {
  const value = args[name];
  return typeof value === "string" ? value : fallback;
}

function boolean(args: Args, name: string): boolean {
  return args[name] === true;
}

export const cli = { parse, help, positionals, string, boolean };
