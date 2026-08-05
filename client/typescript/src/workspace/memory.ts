import type { Json } from "../protocol/serde.js";

/** Text or binary content for one memory workspace file. */
export type MemoryContent = string | Uint8Array | readonly number[];

/** One explicit memory workspace file. */
export type MemoryFile = {
  /** Repository relative file path. */
  readonly path: string;
  /** UTF-8 text content. */
  readonly text?: string;
  /** Binary content. */
  readonly bytes?: Uint8Array | readonly number[];
};

/** Memory workspace source. */
export type MemoryWorkspace = {
  /** In-memory workspace root path. */
  readonly root?: string;
  /** Typed `destack.json` content. */
  readonly config?: Json;
  /** In-memory repository files. */
  readonly files?:
    | Readonly<Record<string, MemoryContent>>
    | readonly MemoryFile[];
};

/** Options for opening an in-memory workspace. */
export type MemoryWorkspaceOptions = {
  /** Memory workspace source. */
  readonly memory: MemoryWorkspace;
};

/** Normalized memory file passed to native local workspace servers. */
export type NativeMemoryFile = {
  /** Repository relative file path. */
  readonly path: string;
  /** UTF-8 text content. */
  readonly text?: string;
  /** Binary content. */
  readonly bytes?: Uint8Array;
};

/** Return whether options describe an in-memory workspace. */
export function isMemoryWorkspaceOptions(
  options: object,
): options is MemoryWorkspaceOptions {
  return "memory" in options;
}

/** Return the root path for one memory workspace. */
export function memoryRoot(options: MemoryWorkspaceOptions): string {
  return options.memory.root ?? "/workspace";
}

/** Return normalized native memory files for one memory workspace. */
export function memoryFiles(
  options: MemoryWorkspaceOptions,
): readonly NativeMemoryFile[] {
  const files = new Map<string, NativeMemoryFile>();
  const config = options.memory.config;

  if (config !== undefined) {
    files.set("destack.json", {
      path: "destack.json",
      text: `${JSON.stringify(config)}\n`,
    });
  }

  for (const file of explicitMemoryFiles(options.memory.files ?? {})) {
    if (file.path === "destack.json" && config !== undefined) {
      throw new Error("memory workspace cannot define both config and destack.json");
    }

    files.set(file.path, file);
  }

  return Array.from(files.values());
}

/** Return explicit memory files from either record or array form. */
function explicitMemoryFiles(
  files: Readonly<Record<string, MemoryContent>> | readonly MemoryFile[],
): readonly NativeMemoryFile[] {
  if (Array.isArray(files)) {
    return files.map(memoryFile);
  }

  return Object.entries(files).map(([path, content]) =>
    memoryFile({ path, ...memoryContent(content) }),
  );
}

/** Return one normalized native memory file. */
function memoryFile(file: MemoryFile): NativeMemoryFile {
  if (file.text !== undefined && file.bytes !== undefined) {
    throw new Error("memory file cannot contain both text and bytes");
  }

  if (file.text !== undefined) {
    return {
      path: file.path,
      text: file.text,
    };
  }

  if (file.bytes !== undefined) {
    return {
      path: file.path,
      bytes: memoryBytes(file.bytes),
    };
  }

  throw new Error("memory file must contain text or bytes");
}

/** Return one memory file payload from short record content. */
function memoryContent(content: MemoryContent): Pick<MemoryFile, "text" | "bytes"> {
  if (typeof content === "string") {
    return {
      text: content,
    };
  }

  return {
    bytes: memoryBytes(content),
  };
}

/** Return binary memory content as a stable byte array. */
function memoryBytes(bytes: Uint8Array | readonly number[]): Uint8Array {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }

  return Uint8Array.from(bytes);
}
