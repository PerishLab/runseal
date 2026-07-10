type Value = null | boolean | number | string | Value[] | { [key: string]: Value };

type Picked = {
  current: Value;
  input: string;
};

class Source {
  static parse(json: string | Value): Value {
    return typeof json === "string" ? JSON.parse(json) as Value : json;
  }

  static array(json: string | Value): Value[] {
    const value = this.parse(json);
    if (!Array.isArray(value)) {
      throw new Error("expected JSON array");
    }
    return value;
  }
}

class Field {
  static string(value: Value, field: string): string | undefined {
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
      return undefined;
    }
    const selected = value[field];
    if (selected === undefined) {
      return undefined;
    }
    if (selected === null) {
      return "null";
    }
    if (typeof selected === "string") {
      return selected;
    }
    if (typeof selected === "boolean" || typeof selected === "number") {
      return String(selected);
    }
    return JSON.stringify(selected);
  }
}

class Path {
  static select(value: Value, path: string): Value {
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
}

function get(json: string | Value, path: string): string {
  const selected = Path.select(Source.parse(json), path);
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

function has(json: string | Value, path: string): boolean {
  try {
    Path.select(Source.parse(json), path);
    return true;
  } catch (err) {
    if (err instanceof Error && err.message === "json path missing") {
      return false;
    }
    throw err;
  }
}

function empty(json: string | Value): boolean {
  const value = Source.parse(json);
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

function len(json: string | Value): number {
  const value = Source.parse(json);
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

function find(json: string | Value, field: string, expected: string): string {
  const array = Source.array(json);
  const found = array.find((item) => Field.string(item, field) === expected);
  return found === undefined ? "" : JSON.stringify(found);
}

function filter(json: string | Value, field: string, expected: string[]): string {
  const array = Source.array(json);
  const filtered = array.filter((item) => {
    const actual = Field.string(item, field);
    return actual !== undefined && expected.includes(actual);
  });
  return JSON.stringify(filtered);
}

function pretty(json: string | Value): string {
  return JSON.stringify(Source.parse(json), null, 2);
}

function index(current: Value, input: string, path: string): Picked {
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

function field(current: Value, input: string): Picked {
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
