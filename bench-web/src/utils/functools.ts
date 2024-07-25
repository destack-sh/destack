import { nextTick } from "vue";

export type FilterPrefix<T, Prefix extends string> = T extends `${Prefix}${string}` ? T : never;

export function nowOrNextTick(delay: boolean | undefined, fn: () => void) {
  if (delay) nextTick(() => fn());
  else fn();
}

export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Partial<Record<T, U>>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
}

export function reverseRecordToMap<T extends PropertyKey, U>(input: Partial<Record<T, U>>) {
  return new Map(Object.entries(input).map(([key, value]) => [value, key]));
}

export function cyrb53a(str: string, seed = 0): number {
  // 53-bit cyrb53a hash
  // see https://github.com/bryc/code/blob/master/jshash/experimental/cyrb53.js
  let h1 = 0xdeadbeef ^ seed,
    h2 = 0x41c6ce57 ^ seed;
  for (let i = 0, ch; i < str.length; i++) {
    ch = str.charCodeAt(i);
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

  set() {
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
