import { createHash, type Hash } from "node:crypto";

/** An incremental hash of a byte stream, spent by its one digest call. */
export interface Hasher {
    /** Add bytes to the digest. */
    update(bytes: Uint8Array): void;
    /** Finish the digest and return its bytes. */
    digest(): Uint8Array<ArrayBuffer>;
}

/** An incremental MD5, SHA-1 or SHA-256 digest from node:crypto. */
export class CryptoHasher implements Hasher {
    /** The running hash. */
    readonly #hash: Hash;

    /** Start a digest of the algorithm. */
    constructor(algorithm: "md5" | "sha1" | "sha256") {
        this.#hash = createHash(algorithm);
    }

    /** Add bytes to the digest. */
    update(bytes: Uint8Array): void {
        this.#hash.update(bytes);
    }

    /** Finish the digest and return its bytes. */
    digest(): Uint8Array<ArrayBuffer> {
        return new Uint8Array(this.#hash.digest());
    }
}
