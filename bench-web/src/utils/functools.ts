import { nextTick } from "vue";

/** Turns an object into a stable string (JSON-serializable, keys sorted) */
export function stringify<T>(obj: T): string {
  if (obj === null || typeof obj !== "object") {
    // primitives: use standard JSON stringification
    return JSON.stringify(obj);
  }

  if (Array.isArray(obj)) {
    // array: map each element recursively
    const arrayResult = obj.map((item) => stringify(item));
    return `[${arrayResult.join(",")}]`;
  }

  // object: sort the keys and build the string
  const sortedKeys = Object.keys(obj).sort();
  const result = sortedKeys.map((key) => {
    const value = obj[key as keyof T];
    return `"${key}":${stringify(value)}`;
  });

  return `{${result.join(",")}}`;
}

/** Deep copy an object (must be JSON-serializable) */
export function copy<T>(obj: T): T {
  if (typeof obj != "object") return obj;
  else return JSON.parse(JSON.stringify(obj));
}

export type FilterPrefix<T, Prefix extends string> = T extends `${Prefix}${string}` ? T : never;

export function nowOrNextTick(delay: boolean | undefined, fn: () => void) {
  if (delay) nextTick(() => fn());
  else fn();
}

export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Partial<Record<T, U>>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
}

// 53-bit cyrb53a hash
// see https://github.com/bryc/code/blob/master/jshash/experimental/cyrb53.js
export function cyrb53a(obj: string | any, seed = 0): number {
  if (typeof obj != "string") obj = stringify(obj);
  if (!obj) return 0;
  let h1 = 0xdeadbeef ^ seed,
    h2 = 0x41c6ce57 ^ seed;
  for (let i = 0, ch; i < obj.length; i++) {
    ch = obj.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 0x85ebca77);
    h2 = Math.imul(h2 ^ ch, 0xc2b2ae3d);
  }
  h1 ^= Math.imul(h1 ^ (h2 >>> 15), 0x735a2d97);
  h2 ^= Math.imul(h2 ^ (h1 >>> 15), 0xcaf649a9);
  h1 ^= h2 >>> 16;
  h2 ^= h1 >>> 16;
  return 2097152 * (h2 >>> 0) + (h1 >>> 11);
}

export function roundToDigits(value: number, digits: number): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

const BASE_64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export function encodeB64VLQ(value: number): string {
  if (value == 0) return "A";
  else if (value < 0) throw new Error(`cannot encode negative value: ${value}`);
  else {
    let result = "";
    while (value) {
      result += BASE_64[value & 63];
      value >>= 6;
    }
    return result;
  }
}

export function decodeB64VLQ(value: string): number {
  let result = 0;
  let shift = 0;
  for (const c of value) {
    result += BASE_64.indexOf(c) << shift;
    shift += 6;
  }
  return result;
}

export function onEveryTick(fn: () => void) {
  let running = true;
  function tick() {
    if (running) {
      fn();
      nextTick(tick);
    }
  }
  tick();
  return () => {
    running = false;
  };
}

/**
 * A simple wrapper around Promises to behave like Python's asyncio.Event
 * Can be reset, set, and waited on.
 * */
export class AsyncEvent {
  private _resolve: (() => void) | null = null;
  private _reject: ((reason?: any) => void) | null = null;
  private _promise: Promise<void> | null = null;

  constructor() {
    this._promise = new Promise((resolve, reject) => {
      this._resolve = resolve;
      this._reject = reject;
    });
  }

  async wait() {
    await this._promise;
  }

  resolve() {
    if (this._resolve) {
      this._resolve();
      this._resolve = null;
      this._promise = null;
    }
  }

  reject(reason?: any) {
    if (this._reject) {
      this._reject(reason);
      this._reject = null;
      this._promise = null;
    }
  }

  reset() {
    this._promise = new Promise((resolve, reject) => {
      this._resolve = resolve;
      this._reject = reject;
    });
  }
}

/** Gets a random value from an enum, ignoring the number keys (which are for protobuf). */
export function getRandomEnum<T extends Record<string | number, any>>(anEnum: T): T[keyof T] {
  const enumValues = Object.values(anEnum).filter((v) => typeof v == "number");
  let randomIndex = Math.floor(Math.random() * enumValues.length);
  while (randomIndex == 0) randomIndex = Math.floor(Math.random() * enumValues.length);
  return enumValues[randomIndex] as T[keyof T];
}

/** Asserts that a specific value must not exist (usually for discriminated union like handling) */
export function assertNever(value?: never, msg?: string): never {
  if (msg != null) {
    throw new Error(`${msg}: ${JSON.stringify(value)}`);
  } else {
    throw new Error(`unexpected value: ${JSON.stringify(value)}`);
  }
}

/** Groups items by the given key function. */
export function groupByList<T, K extends string | number>(items: T[], keyFn: (item: T) => K): Record<K, T[]> {
  const result = {} as Record<K, T[]>;
  for (const item of items) {
    const key = keyFn(item);
    if (result[key] == null) {
      result[key] = [];
    }
    result[key].push(item);
  }
  return result;
}

/** Groups items by the given key function uniquely. */
export function groupByScalar<T, K extends string | number>(items: T[], keyFn: (item: T) => K): Record<K, T> {
  const result = {} as Record<K, T>;
  for (const item of items) {
    const key = keyFn(item);
    if (result[key] != null) {
      throw new Error(`duplicate key ${key}`);
    }
    result[key] = item;
  }
  return result;
}
