import {
    WorkspaceClient,
    workspaceService,
} from "../_generated/workspace/client.js";
import type { OpenRootResponse } from "../_generated/workspace/service/root.js";
import {
    Connection,
    type ConnectionOptions,
} from "../rpc/index.js";
import { hasNodeProcess } from "../runtime.js";
import type { NapiWorkspaceOptions } from "../napi.js";
import type { WasmWorkspaceOptions } from "../wasm.js";
import type {
    MemoryContent,
    MemoryFile,
    MemoryWorkspace,
    MemoryWorkspaceOptions,
} from "./memory.js";
import { isMemoryWorkspaceOptions } from "./memory.js";

/** Location of one workspace served by a remote daemon. */
export type RemoteWorkspaceOptions = {
    /** RPC WebSocket endpoint. */
    readonly url: string | URL;
    /** Informational workspace identity. */
    readonly workspace: string;
    /** Root to open, defaulting to the workspace identity. */
    readonly root?: string;
    /** RPC negotiation overrides. */
    readonly connection?: ConnectionOptions;
};

/** Options for opening one local workspace. */
export type LocalWorkspaceOptions = NapiWorkspaceOptions | WasmWorkspaceOptions;

/** Options for opening one local or remote workspace. */
export type WorkspaceOptions = LocalWorkspaceOptions | RemoteWorkspaceOptions;

/** One opened root over a composable workspace RPC client. */
export class Workspace {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Complete generated workspace service client. */
    readonly client: WorkspaceClient;
    /** Informational workspace identity. */
    readonly workspace: string;
    /** Canonical opened root. */
    readonly root: string;
    /** Revision observed while opening this root. */
    readonly openedRevision: OpenRootResponse["revision"];

    /** Create one already opened workspace root. */
    private constructor(
        connection: Connection,
        workspace: string,
        opened: OpenRootResponse,
    ) {
        this.connection = connection;
        this.client = new WorkspaceClient(connection);
        this.workspace = workspace;
        this.root = opened.root;
        this.openedRevision = opened.revision;
    }

    /** Open one root over an already negotiated connection. */
    static async open(
        connection: Connection,
        workspace: string,
        root: string = workspace,
    ): Promise<Workspace> {
        const client = new WorkspaceClient(connection);
        const response = await client.openRoot({ root });

        return new Workspace(connection, workspace, response.value);
    }
}

/** Connect to a remote daemon and open one workspace root. */
export async function openRemoteWorkspace(
    options: RemoteWorkspaceOptions,
): Promise<Workspace> {
    const connection = await Connection.connectWebSocket(
        options.url,
        [workspaceService],
        options.connection,
    );

    return Workspace.open(
        connection,
        options.workspace,
        options.root ?? options.workspace,
    );
}

/** Open a workspace through its selected remote or local host. */
export async function openWorkspace(options: WorkspaceOptions): Promise<Workspace> {
    if (isRemoteWorkspaceOptions(options)) {
        return openRemoteWorkspace(options);
    }
    if (hasNodeProcess()) {
        const { openNapiWorkspace } = await import("../napi.js");

        return openNapiWorkspace(options);
    }
    if (!isMemoryWorkspaceOptions(options)) {
        throw new Error("physical workspaces require a Node.js host");
    }

    const { openWasmWorkspace } = await import("../wasm.js");

    return openWasmWorkspace(options);
}

/** Open one local workspace through the best available host. */
export function openLocalWorkspace(options: LocalWorkspaceOptions): Promise<Workspace> {
    return openWorkspace(options);
}

/** Return whether workspace options select a remote endpoint. */
function isRemoteWorkspaceOptions(options: WorkspaceOptions): options is RemoteWorkspaceOptions {
    return "url" in options;
}

export { WorkspaceClient, workspaceService };
export type {
    MemoryContent,
    MemoryFile,
    MemoryWorkspace,
    MemoryWorkspaceOptions,
};
