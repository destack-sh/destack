import { EmbeddedTransport, Connection } from "./protocol/connection/index.js";
import {
  RemoteWorkspace,
  openRemoteWorkspace,
  type RemoteWorkspaceOptions,
} from "./protocol/workspace.js";

/** Options for opening a local NAPI workspace. */
export type NapiWorkspaceOptions = Omit<
  RemoteWorkspaceOptions,
  "connection" | "url"
>;

/** Native NAPI module shape consumed by embedded workspace transport. */
type NapiModule = {
  readonly LocalWorkspaceServer: {
    readonly open: (workspace: string) => LocalWorkspaceServer;
  };
};

/** Native local workspace server. */
type LocalWorkspaceServer = {
  readonly dispatch: (bytes: Uint8Array) => readonly unknown[];
};

/** Open a workspace backed by the native Node bridge. */
export async function openNapiWorkspace(
  options: NapiWorkspaceOptions,
): Promise<RemoteWorkspace> {
  const napi =
    (await import("@destack/language-napi")) as unknown as NapiModule;
  const server = napi.LocalWorkspaceServer.open(options.workspace);
  const transport = new EmbeddedTransport(new NapiServer(server));
  const connection = new Connection(transport);

  return openRemoteWorkspace({ ...options, connection });
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
    return this.#server.dispatch(bytes);
  }
}
