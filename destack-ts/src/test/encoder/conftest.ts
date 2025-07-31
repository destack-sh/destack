import { BinaryReader, BinaryWriter, type Encoder } from "@destack/language";

// Types to silence in logs (avoid circular references and noise)
const SILENCE_TYPES = new Set(["Session", "Graph", "Encoder", "BinaryReader", "BinaryWriter"]);

function shouldSilence(arg: any): boolean {
  if (!arg) return false;
  const name = arg.constructor?.name;
  return name && SILENCE_TYPES.has(name);
}

function formatArg(arg: any): string {
  if (shouldSilence(arg)) {
    return `<${arg.constructor.name}>`;
  }
  if (typeof arg === "bigint") {
    return `${arg}n`;
  }
  return JSON.stringify(arg);
}

/**
 * A BinaryWriter that logs all writes to console.
 */
export class LoggingBinaryWriter extends BinaryWriter {
  constructor(initialSize?: number) {
    super(initialSize);

    // dynamically wrap all write_* methods
    const proto = Object.getPrototypeOf(this);
    const methodNames = Object.getOwnPropertyNames(proto);

    for (const methodName of methodNames) {
      if (methodName.startsWith("write")) {
        const originalMethod = proto[methodName];
        if (typeof originalMethod === "function") {
          (this as any)[methodName] = this.makeWriteLoggingWrapper(methodName, originalMethod);
        }
      }
    }
  }

  private makeWriteLoggingWrapper(methodName: string, originalMethod: Function) {
    return (...args: any[]) => {
      const filteredArgs = args.filter((arg) => !shouldSilence(arg));
      console.log(`${methodName}`, ...filteredArgs.map(formatArg));
      return originalMethod.apply(this, args);
    };
  }
}

/**
 * A BinaryReader that logs all reads to console.
 */
export class LoggingBinaryReader extends BinaryReader {
  constructor(data: Uint8Array) {
    super(data);

    // dynamically wrap all read_* and peek_* methods
    const proto = Object.getPrototypeOf(this);
    const methodNames = Object.getOwnPropertyNames(proto);

    for (const methodName of methodNames) {
      if (methodName.startsWith("read") || methodName.startsWith("peek")) {
        const originalMethod = proto[methodName];
        if (typeof originalMethod === "function") {
          (this as any)[methodName] = this.makeReadLoggingWrapper(methodName, originalMethod);
        }
      }
    }
  }

  private makeReadLoggingWrapper(methodName: string, originalMethod: Function) {
    return (...args: any[]) => {
      const result = originalMethod.apply(this, args);
      const filteredArgs = args.filter((arg) => !shouldSilence(arg));
      console.log(`${methodName}`, ...filteredArgs.map(formatArg), "->", formatArg(result));
      return result;
    };
  }
}

/**
 * Wrap an encoder instance to log all method calls.
 */
export function wrapEncoder<T>(encoder: Encoder<T>): Encoder<T> {
  const wrapped = Object.create(encoder);

  // Get all methods from the encoder
  const methods = [
    "packObject",
    "packObjectBinary",
    "unpackObject",
    "unpackObjectBinary",
    "packType",
    "packTypeBinary",
    "unpackType",
    "unpackTypeBinary",
    "packValue",
    "packValueBytes",
    "unpackValue",
    "unpackValueBytes",
  ];

  for (const methodName of methods) {
    if (typeof (encoder as any)[methodName] === "function") {
      wrapped[methodName] = makeLoggingWrapper(
        methodName,
        (encoder as any)[methodName].bind(encoder),
      );
    }
  }

  return wrapped;
}

function makeLoggingWrapper(methodName: string, originalMethod: Function) {
  return (...args: any[]) => {
    const argStrings: string[] = [];

    for (let i = 0; i < args.length; i++) {
      if (!shouldSilence(args[i])) {
        argStrings.push(formatArg(args[i]));
      }
    }

    console.log(`${methodName}(${argStrings.join(", ")})`);
    const result = originalMethod(...args);
    console.log(`${methodName} -> ${formatArg(result)}`);
    return result;
  };
}
