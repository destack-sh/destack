import { EmbeddedTransport, Connection } from "./protocol/connection/index.js";
import {
  RemoteWorkspace,
  openRemoteWorkspace,
  type RemoteWorkspaceOptions,
} from "./protocol/workspace.js";

/** Options for opening a local WASM workspace. */
export type WasmWorkspaceOptions = Omit<
  RemoteWorkspaceOptions,
  "connection" | "url"
>;

/** Native WASM module shape consumed by embedded workspace transport. */
type WasmModule = {
  readonly default: () => Promise<unknown>;
  readonly LocalWorkspaceServer: {
    readonly open: (workspace: string) => LocalWorkspaceServer;
  };
};

/** Native local workspace server. */
type LocalWorkspaceServer = {
  readonly dispatch: (bytes: Uint8Array) => readonly unknown[];
};

/** Open a workspace backed by the WebAssembly bridge. */
export async function openWasmWorkspace(
  options: WasmWorkspaceOptions,
): Promise<RemoteWorkspace> {
  const wasm =
    (await import("@destack/language-wasm")) as unknown as WasmModule;
  await wasm.default();

  const server = wasm.LocalWorkspaceServer.open(options.workspace);
  const transport = new EmbeddedTransport(new WasmServer(server));
  const connection = new Connection(transport);

  return openRemoteWorkspace({ ...options, connection });
}

/** Embedded server adapter for WASM native objects. */
class WasmServer {
  readonly #server: LocalWorkspaceServer;

  /** Create one WASM server adapter. */
  constructor(server: LocalWorkspaceServer) {
    this.#server = server;
  }

  /** Dispatch one encoded protocol frame. */
  dispatch(bytes: Uint8Array): readonly unknown[] {
    return Array.from(this.#server.dispatch(bytes));
  }
}
