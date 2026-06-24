import { EmbeddedTransport, Connection } from "./protocol/connection/index.js";
import {
  RemoteWorkspace,
  openRemoteWorkspace,
  type RemoteWorkspaceOptions,
} from "./protocol/workspace.js";
import {
  isMemoryWorkspaceOptions,
  memoryFiles,
  memoryRoot,
  type MemoryWorkspaceOptions,
} from "./workspace/memory.js";

/** Options for opening a local NAPI workspace. */
export type PathWorkspaceOptions = Omit<RemoteWorkspaceOptions, "connection" | "url">;

/** Options for opening a local NAPI workspace. */
export type NapiWorkspaceOptions = PathWorkspaceOptions | MemoryWorkspaceOptions;

/** Native NAPI module shape consumed by embedded workspace transport. */
type NapiModule = {
  readonly LocalWorkspaceServer: {
    readonly open: (workspace: string) => LocalWorkspaceServer;
    readonly memory: (
      root: string,
      files: readonly NapiMemoryFile[],
    ) => LocalWorkspaceServer;
  };
};

/** Memory file shape accepted by the generated NAPI binding. */
type NapiMemoryFile = {
  /** Repository relative file path. */
  readonly path: string;
  /** UTF-8 text content. */
  readonly text?: string;
  /** Binary content. */
  readonly bytes?: readonly number[];
};

/** Native local workspace server. */
type LocalWorkspaceServer = {
  readonly dispatch: (bytes: readonly number[]) => readonly unknown[];
};

/** Open a workspace backed by the native Node bridge. */
export async function openNapiWorkspace(
  options: NapiWorkspaceOptions,
): Promise<RemoteWorkspace> {
  const napi =
    (await import("@destack/language-napi")) as unknown as NapiModule;

  const server = isMemoryWorkspaceOptions(options)
    ? napi.LocalWorkspaceServer.memory(memoryRoot(options), napiMemoryFiles(options))
    : napi.LocalWorkspaceServer.open(options.workspace);
  const transport = new EmbeddedTransport(new NapiServer(server));
  const connection = new Connection(transport);
  const workspace = isMemoryWorkspaceOptions(options) ? memoryRoot(options) : options.workspace;

  return openRemoteWorkspace({ ...options, workspace, connection });
}

/** Embedded server adapter for NAPI native objects. */
class NapiServer {
  readonly #server: LocalWorkspaceServer;

  /** Create one NAPI server adapter. */
  constructor(server: LocalWorkspaceServer) {
    this.#server = server;
  }

  /** Dispatch one encoded protocol frame. */
  dispatch(bytes: Uint8Array): readonly unknown[] {
    return this.#server.dispatch(Array.from(bytes));
  }
}

/** Return memory files in the shape accepted by NAPI. */
function napiMemoryFiles(options: MemoryWorkspaceOptions): readonly NapiMemoryFile[] {
  return memoryFiles(options).map((file) => ({
    path: file.path,
    text: file.text,
    bytes: file.bytes === undefined ? undefined : Array.from(file.bytes),
  }));
}
