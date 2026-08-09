import { workspaceService } from "./_generated/workspace/client.js";
import { Connection, EmbeddedTransport, type EmbeddedSession } from "./rpc/index.js";
import {
    memoryFiles,
    memoryRoot,
    type MemoryWorkspaceOptions,
    type NativeMemoryFile,
} from "./workspace/memory.js";
import { Workspace } from "./workspace/workspace.js";

/** Options for opening one in-memory WebAssembly workspace. */
export type WasmWorkspaceOptions = MemoryWorkspaceOptions;

/** WebAssembly module shape consumed by this client. */
type WasmModule = {
    /** Initialize the generated WebAssembly module. */
    readonly default: () => Promise<unknown>;
    /** In-process workspace RPC session constructor. */
    readonly WorkspaceSession: {
        /** Open one in-memory workspace. */
        readonly memory: (root: string, files: readonly NativeMemoryFile[]) => NativeSession;
    };
};

/** Native WebAssembly RPC session. */
type NativeSession = {
    /** Dispatch one RPC message. */
    readonly dispatch: (bytes: Uint8Array) => readonly unknown[];
    /** Poll ready RPC calls. */
    readonly poll: () => readonly unknown[];
    /** Return whether a cooperative RPC call requested another poll. */
    readonly isReady: () => boolean;
    /** Close this session. */
    readonly close: () => void;
};

/** Open an in-memory workspace backed by WebAssembly. */
export async function openWasmWorkspace(options: WasmWorkspaceOptions): Promise<Workspace> {
    const wasm = (await import("@destack/language-wasm")) as unknown as WasmModule;
    await wasm.default();

    const workspace = memoryRoot(options);
    const native = wasm.WorkspaceSession.memory(workspace, memoryFiles(options));
    const transport = new EmbeddedTransport(new WasmSession(native));
    const connection = new Connection(transport);
    await connection.handshake([workspaceService]);

    return Workspace.open(connection, workspace);
}

/** Adapter between wasm-bindgen and the generic embedded transport. */
class WasmSession implements EmbeddedSession {
    readonly #session: NativeSession;

    /** Create one WebAssembly session adapter. */
    constructor(session: NativeSession) {
        this.#session = session;
    }

    /** Dispatch one complete inbound RPC message. */
    dispatch(bytes: Uint8Array): readonly unknown[] {
        return this.#session.dispatch(bytes);
    }

    /** Poll cooperatively ready RPC calls. */
    poll(): readonly unknown[] {
        return this.#session.poll();
    }

    /** Return whether a cooperative RPC call requested another poll. */
    isReady(): boolean {
        return this.#session.isReady();
    }

    /** Close this WebAssembly session. */
    close(): void {
        this.#session.close();
    }
}
