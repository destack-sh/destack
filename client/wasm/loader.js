import initializeWasm, {
    WorkspaceSession,
    initSync as initializeWasmSync,
    initializeBuild,
    version,
} from "./dist/destack_wasm.js";

export { WorkspaceSession, version };

/** Initialize the WebAssembly module from its exact bytes. */
export default function initialize(moduleOrPath) {
    return initializeExact(moduleOrPath);
}

/** Initialize the WebAssembly module synchronously from its exact bytes. */
export function initSync(module) {
    const bytes = copyBytes(module);
    const output = initializeWasmSync({ module: bytes });
    initializeBuild(bytes);

    return output;
}

/** Fetch, instantiate, and identify one exact WebAssembly module. */
async function initializeExact(moduleOrPath) {
    const source = moduleOrPath ?? new URL("./dist/destack_wasm_bg.wasm", import.meta.url);
    const bytes = await loadBytes(source);
    const output = await initializeWasm({ module_or_path: bytes });
    initializeBuild(bytes);

    return output;
}

/** Load exact bytes from one supported asynchronous module source. */
async function loadBytes(source) {
    const resolved = await source;

    // consume an existing response exactly once
    if (typeof Response === "function" && resolved instanceof Response) {
        if (!resolved.ok) {
            throw new Error(`WebAssembly module request failed with status ${resolved.status}`);
        }

        return new Uint8Array(await resolved.arrayBuffer());
    }

    // fetch URL shaped module sources before instantiation
    if (
        typeof resolved === "string" ||
        (typeof URL === "function" && resolved instanceof URL) ||
        (typeof Request === "function" && resolved instanceof Request)
    ) {
        const response = await fetch(resolved);
        if (!response.ok) {
            throw new Error(`WebAssembly module request failed with status ${response.status}`);
        }

        return new Uint8Array(await response.arrayBuffer());
    }

    return copyBytes(resolved);
}

/** Copy one byte source so instantiation and identity observe identical bytes. */
function copyBytes(source) {
    if (typeof WebAssembly === "object" && source instanceof WebAssembly.Module) {
        throw new TypeError("exact WebAssembly module bytes are required for build identity");
    }
    if (source instanceof ArrayBuffer) {
        return new Uint8Array(source.slice(0));
    }
    if (ArrayBuffer.isView(source)) {
        const bytes = new Uint8Array(source.buffer, source.byteOffset, source.byteLength);

        return bytes.slice();
    }

    throw new TypeError("WebAssembly module must be a URL, response, or byte source");
}
