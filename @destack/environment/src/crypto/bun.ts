import { createHash } from "node:crypto";
import { Crypto } from "./crypto.ts";

/** The node:crypto hash names of the WebCrypto digest algorithms. */
const HASHES = new Map([
    ["SHA-1", "sha1"],
    ["SHA-256", "sha256"],
    ["SHA-384", "sha384"],
    ["SHA-512", "sha512"],
]);

/** The host's WebCrypto operations, hashing streams chunk by chunk with its native hash. */
export const BunCrypto: Crypto = Crypto.of(crypto.subtle, async (algorithm, chunks) => {
    // refuse an algorithm without a name, and one the native hash lacks
    if (typeof algorithm !== "string" && !("name" in algorithm)) {
        throw new TypeError("cannot hash a stream with an unnamed algorithm");
    }
    const name = typeof algorithm === "string" ? algorithm : algorithm.name;
    const native = HASHES.get(name.toUpperCase());
    if (native === undefined) {
        throw new TypeError(`cannot hash a stream with ${name}`);
    }

    // hash each chunk as it arrives
    const hash = createHash(native);
    for await (const chunk of chunks) {
        hash.update(chunk);
    }

    return Uint8Array.from(hash.digest()).buffer;
});
