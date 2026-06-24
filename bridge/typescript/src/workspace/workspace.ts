import { hasNodeProcess } from "../runtime.js";
import {
  RemoteWorkspace,
  openRemoteWorkspace,
  type RemoteWorkspaceOptions,
  type WatchOptionsInit,
  type Workspace,
} from "../protocol/workspace.js";
import { EmbeddedTransport } from "../protocol/connection/index.js";
import type { NapiWorkspaceOptions } from "../napi.js";
import type { WasmWorkspaceOptions } from "../wasm.js";
import type {
  MemoryContent,
  MemoryFile,
  MemoryWorkspace,
  MemoryWorkspaceOptions,
} from "./memory.js";

/** Options for opening a local workspace. */
export type LocalWorkspaceOptions = NapiWorkspaceOptions | WasmWorkspaceOptions;

/** Options for opening a workspace. */
export type WorkspaceOptions = LocalWorkspaceOptions | RemoteWorkspaceOptions;

export {
  EmbeddedTransport,
  RemoteWorkspace,
  openRemoteWorkspace,
  type WatchOptionsInit,
  type MemoryContent,
  type MemoryFile,
  type MemoryWorkspace,
  type MemoryWorkspaceOptions,
  type Workspace,
};

/** Open a workspace backed by the best available transport. */
export async function openWorkspace(
  options: WorkspaceOptions,
): Promise<Workspace> {
  if (isRemoteWorkspaceOptions(options)) {
    return openRemoteWorkspace(options);
  }

  if (hasNodeProcess()) {
    const { openNapiWorkspace } = await import("../napi.js");

    return openNapiWorkspace(options);
  }

  const { openWasmWorkspace } = await import("../wasm.js");

  return openWasmWorkspace(options);
}

/** Open a local workspace backed by the best available embedded transport. */
export async function openLocalWorkspace(
  options: LocalWorkspaceOptions,
): Promise<Workspace> {
  return openWorkspace(options);
}

/** Return whether workspace options name a remote transport. */
function isRemoteWorkspaceOptions(
  options: WorkspaceOptions,
): options is RemoteWorkspaceOptions {
  return "connection" in options || "url" in options;
}
