type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

type Picked = {
  current: JsonValue;
  input: string;
};

function parseInput(json: string | JsonValue): JsonValue {
  return typeof json === "string" ? JSON.parse(json) as JsonValue : json;
}

function get(json: string | JsonValue, path: string): string {
  const selected = selectPath(parseInput(json), path);
  if (selected === null) {
    return "";
  }
  switch (typeof selected) {
    case "string":
      return selected;
    case "boolean":
    case "number":
      return String(selected);
    case "object":
      return JSON.stringify(selected);
  }
}

function has(json: string | JsonValue, path: string): boolean {
  try {
    selectPath(parseInput(json), path);
    return true;
  } catch (err) {
    if (err instanceof Error && err.message === "json path missing") {
      return false;
    }
    throw err;
  }
}

function empty(json: string | JsonValue): boolean {
  const value = parseInput(json);
  if (value === null) {
    return true;
  }
  if (typeof value === "string" || Array.isArray(value)) {
    return value.length === 0;
  }
  if (typeof value === "object") {
    return Object.keys(value).length === 0;
  }
  return false;
}

function len(json: string | JsonValue): number {
  const value = parseInput(json);
  if (value === null) {
    return 0;
  }
  if (typeof value === "string" || Array.isArray(value)) {
    return value.length;
  }
  if (typeof value === "object") {
    return Object.keys(value).length;
  }
  return 1;
}

function find(json: string | JsonValue, field: string, expected: string): string {
  const array = parseArray(json);
  const found = array.find((item) => fieldString(item, field) === expected);
  return found === undefined ? "" : JSON.stringify(found);
}

function filter(json: string | JsonValue, field: string, expected: string[]): string {
  const array = parseArray(json);
  const filtered = array.filter((item) => {
    const actual = fieldString(item, field);
    return actual !== undefined && expected.includes(actual);
  });
  return JSON.stringify(filtered);
}

function pretty(json: string | JsonValue): string {
  return JSON.stringify(parseInput(json), null, 2);
}

function parseArray(json: string | JsonValue): JsonValue[] {
  const value = parseInput(json);
  if (!Array.isArray(value)) {
    throw new Error("expected JSON array");
  }
  return value;
}

function fieldString(value: JsonValue, field: string): string | undefined {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return undefined;
  }
  const fieldValue = value[field];
  if (fieldValue === undefined) {
    return undefined;
  }
  if (fieldValue === null) {
    return "null";
  }
  if (typeof fieldValue === "string") {
    return fieldValue;
  }
  if (typeof fieldValue === "boolean" || typeof fieldValue === "number") {
    return String(fieldValue);
  }
  return JSON.stringify(fieldValue);
}

function selectPath(value: JsonValue, path: string): JsonValue {
  let input = path.startsWith(".") ? path.slice(1) : path;
  if (input === "") {
    throw new Error("json path cannot be empty");
  }
  let current = value;
  while (input !== "") {
    if (input.startsWith("[")) {
      const picked = index(current, input, path);
      current = picked.current;
      input = picked.input;
      continue;
    }
    const picked = field(current, input);
    current = picked.current;
    input = picked.input;
  }
  return current;
}

function index(current: JsonValue, input: string, path: string): Picked {
  const end = input.indexOf("]");
  if (end === -1) {
    throw new Error(`unsupported json path: ${path}`);
  }
  const slot = Number(input.slice(1, end));
  if (!Number.isInteger(slot) || slot < 0) {
    throw new Error(`invalid json path index: ${input.slice(1, end)}`);
  }
  if (!Array.isArray(current) || current[slot] === undefined) {
    throw new Error("json path missing");
  }
  return { current: current[slot], input: rest(input, end + 1) };
}

function field(current: JsonValue, input: string): Picked {
  const dot = input.indexOf(".");
  const bracket = input.indexOf("[");
  const choices = [dot, bracket].filter((at) => at >= 0);
  const end = choices.length === 0 ? input.length : Math.min(...choices);
  const key = input.slice(0, end);
  if (!/^[A-Za-z0-9_-]+$/.test(key)) {
    throw new Error(`unsupported json path field: ${key}`);
  }
  if (current === null || typeof current !== "object" || Array.isArray(current)) {
    throw new Error("json path missing");
  }
  const selected = current[key];
  if (selected === undefined) {
    throw new Error("json path missing");
  }
  return { current: selected, input: rest(input, end) };
}

function rest(input: string, end: number): string {
  const next = input.slice(end);
  return next.startsWith(".") ? next.slice(1) : next;
}

export const json = {
  get,
  has,
  empty,
  len,
  find,
  filter,
  pretty,
};
