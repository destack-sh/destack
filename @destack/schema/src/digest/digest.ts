import * as schema from "../validate/index.ts";
import { defineSchema } from "../declare/schema.ts";
import { canonicalize } from "../json/json.ts";

/** The runtime's `node:crypto` on Bun, Node, Deno and workerd, absent in browsers. */
const NODE_CRYPTO = loadNodeCrypto();

/** The schema of a SHA-256 digest as lowercase hexadecimal. */
const digestSchema = defineSchema(schema.string().regex(/^[0-9a-f]{64}$/u));

/** A SHA-256 digest as lowercase hexadecimal. */
export type Digest = schema.Infer<typeof digestSchema>;

/** A SHA-256 digest as lowercase hexadecimal. */
export const Digest = Object.assign(digestSchema, {
    /** Hash bytes, or the UTF-8 bytes of text. */
    of: hash,

    /** Hash a JSON value's canonical form. */
    json(value: unknown): Promise<Digest> {
        return hash(canonicalize(value));
    },
});

/** The `node:crypto` hashing a runtime offers. */
interface NodeCrypto {
    /** Start a hash of an algorithm. */
    readonly createHash: (algorithm: "sha256") => NodeHash;
}

/** A running `node:crypto` hash. */
interface NodeHash {
    /** Add bytes or the UTF-8 bytes of text. */
    readonly update: (content: string | Uint8Array) => NodeHash;
    /** Finish the hash as lowercase hexadecimal. */
    readonly digest: (encoding: "hex") => string;
}

/** Load the runtime's `node:crypto` through its built-in modules, absent in browsers. */
function loadNodeCrypto(): NodeCrypto | undefined {
    // read the runtime's process, absent in browsers
    const scope: object = globalThis;
    const runtime = "process" in scope ? scope.process : undefined;

    // load node:crypto through its built-in modules
    const loaded = isModuleLoader(runtime) ? runtime.getBuiltinModule("node:crypto") : undefined;

    return isNodeCrypto(loaded) ? loaded : undefined;
}

/** Report whether a runtime loads built-in modules by name. */
function isModuleLoader(
    value: unknown,
): value is { readonly getBuiltinModule: (id: string) => unknown } {
    return (
        typeof value === "object" &&
        value !== null &&
        "getBuiltinModule" in value &&
        typeof value.getBuiltinModule === "function"
    );
}

/** Report whether a module hashes as `node:crypto` does. */
function isNodeCrypto(value: unknown): value is NodeCrypto {
    return (
        typeof value === "object" &&
        value !== null &&
        "createHash" in value &&
        typeof value.createHash === "function"
    );
}

/** Hash bytes, or the UTF-8 bytes of text, with SHA-256. */
async function hash(content: string | Uint8Array<ArrayBuffer>): Promise<Digest> {
    // hash in place, about 1 µs per KB, which a Web Crypto round trip through the event loop exceeds
    if (NODE_CRYPTO !== undefined) {
        return NODE_CRYPTO.createHash("sha256").update(content).digest("hex");
    }

    // hash through Web Crypto in a browser
    const bytes = typeof content === "string" ? new TextEncoder().encode(content) : content;

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}

/** A committed Git object identifier: a SHA-1 or SHA-256 digest in hexadecimal. */
export const Commit = defineSchema(schema.string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/u));
/** A committed Git object identifier. */
export type Commit = schema.Infer<typeof Commit>;
