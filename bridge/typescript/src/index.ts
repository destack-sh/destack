const PACKAGE_VERSION = "0.55.3";

/** Supported backend kinds. */
export type ClientBackend = "napi" | "wasm";

/** A client surface for Destack. */
export interface DestackClient {
    /** The selected backend. */
    readonly backend: ClientBackend;
    /** Return package version. */
    version(): string;
}

class DefaultClient implements DestackClient {
    public constructor(public readonly backend: ClientBackend) {}

    public version(): string {
        return PACKAGE_VERSION;
    }
}

/** Detect the default backend for the current runtime. */
export function detectDefaultBackend(): ClientBackend {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;
    return processValue?.versions?.node == null ? "wasm" : "napi";
}

/** Create a client instance. */
export async function createClient(
    backend: ClientBackend = detectDefaultBackend(),
): Promise<DestackClient> {
    return new DefaultClient(backend);
}

/** Create a client pinned to the Node-API backend. */
export async function createNapiClient(): Promise<DestackClient> {
    return createClient("napi");
}

/** Create a client pinned to the WebAssembly backend. */
export async function createWasmClient(): Promise<DestackClient> {
    return createClient("wasm");
}
