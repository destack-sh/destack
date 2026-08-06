import { workspaceService } from "./_generated/workspace/client.js";
import { Connection, EmbeddedTransport, type EmbeddedSession } from "./rpc/index.js";
import {
    isMemoryWorkspaceOptions,
    memoryFiles,
    memoryRoot,
    type MemoryWorkspaceOptions,
} from "./workspace/memory.js";
import { Workspace } from "./workspace/workspace.js";

/** Options for opening one physical native workspace. */
export type PathWorkspaceOptions = {
    /** Physical workspace path. */
    readonly workspace: string;
    /** Root to open, defaulting to the workspace path. */
    readonly root?: string;
};

/** Options for opening one native workspace. */
export type NapiWorkspaceOptions = PathWorkspaceOptions | MemoryWorkspaceOptions;

/** Native N-API module shape consumed by this client. */
type NapiModule = {
    /** In-process workspace RPC session constructor. */
    readonly WorkspaceSession: {
        /** Open one physical workspace. */
        readonly open: (workspace: string) => NativeSession;
        /** Open one in-memory workspace. */
        readonly memory: (root: string, files: readonly NapiMemoryFile[]) => NativeSession;
    };
};

/** In-memory file shape accepted by N-API. */
type NapiMemoryFile = {
    /** Repository relative file path. */
    readonly path: string;
    /** UTF-8 text content. */
    readonly text?: string;
    /** Binary content. */
    readonly bytes?: readonly number[];
};

/** Native session using N-API byte arrays. */
type NativeSession = {
    /** Dispatch one RPC message. */
    readonly dispatch: (bytes: readonly number[]) => readonly unknown[];
    /** Poll ready RPC calls. */
    readonly poll: () => readonly unknown[];
    /** Return whether a cooperative RPC call requested another poll. */
    readonly isReady: () => boolean;
    /** Register a callback invoked when a cooperative call becomes ready. */
    readonly onReady: (wake: () => void) => void;
    /** Close this session. */
    readonly close: () => void;
};

/** Open a workspace backed by the native Node host. */
export async function openNapiWorkspace(options: NapiWorkspaceOptions): Promise<Workspace> {
    const napi = (await import("@destack/language-napi")) as unknown as NapiModule;
    const isMemory = isMemoryWorkspaceOptions(options);
    const workspace = isMemory ? memoryRoot(options) : options.workspace;
    const root = isMemory ? workspace : (options.root ?? workspace);
    const native = isMemory
        ? napi.WorkspaceSession.memory(workspace, napiMemoryFiles(options))
        : napi.WorkspaceSession.open(workspace);
    const transport = new EmbeddedTransport(new NapiSession(native));
    const connection = new Connection(transport);
    await connection.handshake([workspaceService]);

    return Workspace.open(connection, workspace, root);
}

/** Adapter between N-API arrays and the generic embedded transport. */
class NapiSession implements EmbeddedSession {
    readonly #session: NativeSession;

    /** Create one N-API session adapter. */
    constructor(session: NativeSession) {
        this.#session = session;
    }

    /** Dispatch one complete inbound RPC message. */
    dispatch(bytes: Uint8Array): readonly unknown[] {
        return this.#session.dispatch(Array.from(bytes));
    }

    /** Poll cooperatively ready RPC calls. */
    poll(): readonly unknown[] {
        return this.#session.poll();
    }

    /** Return whether a cooperative RPC call requested another poll. */
    isReady(): boolean {
        return this.#session.isReady();
    }

    /** Register a callback invoked when a cooperative call becomes ready. */
    onReady(wake: () => void): void {
        this.#session.onReady(wake);
    }

    /** Close this native session. */
    close(): void {
        this.#session.close();
    }
}

/** Return memory files in the shape accepted by N-API. */
function napiMemoryFiles(options: MemoryWorkspaceOptions): readonly NapiMemoryFile[] {
    return memoryFiles(options).map((file) => ({
        path: file.path,
        text: file.text,
        bytes: file.bytes === undefined ? undefined : Array.from(file.bytes),
    }));
}
