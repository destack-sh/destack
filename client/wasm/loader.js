import initializeWasm, {
    WorkspaceSession,
    initSync as initializeWasmSync,
    version,
} from "./dist/destack_wasm.js";

export { WorkspaceSession, version };

/** Initialize the WebAssembly module. */
export default function initialize(moduleOrPath) {
    if (moduleOrPath === undefined) {
        return initializeWasm();
    }

    return initializeWasm({ module_or_path: moduleOrPath });
}

/** Initialize the WebAssembly module synchronously. */
export function initSync(module) {
    return initializeWasmSync({ module });
}
