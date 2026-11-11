import { createHash } from "node:crypto";
import { Crypto } from "./crypto.ts";

/** The node:crypto hash names of the WebCrypto digest algorithms. */
const HASHES = new Map([
    ["SHA-1", "sha1"],
    ["SHA-256", "sha256"],
    ["SHA-384", "sha384"],
    ["SHA-512", "sha512"],
]);

/** The host's WebCrypto operations, hashing bytes and streams in process with its native hash. */
export const BunCrypto: Crypto = { ...Crypto.of(crypto.subtle, digest), digest };

/** Hash bytes, or a stream chunk by chunk, in process with the native hash. */
async function digest(
    algorithm: Parameters<Crypto["digest"]>[0],
    data: Parameters<Crypto["digest"]>[1],
): Promise<ArrayBuffer> {
    // refuse an algorithm without a name, and one the native hash lacks
    if (typeof algorithm !== "string" && !("name" in algorithm)) {
        throw new TypeError("cannot hash with an unnamed algorithm");
    }
    const name = typeof algorithm === "string" ? algorithm : algorithm.name;
    const native = HASHES.get(name.toUpperCase());
    if (native === undefined) {
        throw new TypeError(`cannot hash with '${name}'`);
    }

    // hash the bytes, or each chunk as it arrives
    const hash = createHash(native);
    if (Symbol.asyncIterator in data) {
        for await (const chunk of data) {
            hash.update(chunk);
        }
    } else {
        hash.update(
            ArrayBuffer.isView(data)
                ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
                : new Uint8Array(data),
        );
    }

    return Uint8Array.from(hash.digest()).buffer;
}
