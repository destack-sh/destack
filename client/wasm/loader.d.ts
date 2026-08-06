export {
    LocalWorkspaceServer,
    type InitOutput,
    type SyncInitInput,
    version,
} from "./dist/destack_wasm.js";

/** Exact WebAssembly module source accepted by the asynchronous loader. */
export type InitInput = RequestInfo | URL | Response | BufferSource;

/** Initialize the WebAssembly module synchronously from its exact bytes. */
export function initSync(module: BufferSource): import("./dist/destack_wasm.js").InitOutput;

/** Initialize the WebAssembly module from its exact bytes. */
export default function initialize(
    moduleOrPath?: InitInput | Promise<InitInput>,
): Promise<import("./dist/destack_wasm.js").InitOutput>;
