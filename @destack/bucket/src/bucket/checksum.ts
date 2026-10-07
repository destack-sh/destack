/** Supported file checksum algorithms. */
export const CHECKSUM_ALGORITHMS = ["md5", "sha1", "sha256", "sha384", "sha512"] as const;

/** A supported checksum algorithm. */
export type ChecksumAlgorithm = (typeof CHECKSUM_ALGORITHMS)[number];

/** Stored content checksums. */
export class BucketChecksums {
    /** The stored hexadecimal digests. */
    readonly #digests: Partial<Record<ChecksumAlgorithm, string>>;

    /** Retain the stored checksums. */
    constructor(digests: Partial<Record<ChecksumAlgorithm, string>> = {}) {
        this.#digests = { ...digests };
    }

    /** The MD5 checksum. */
    get md5(): ArrayBuffer | undefined {
        return this.#read("md5");
    }
    /** The SHA-1 checksum. */
    get sha1(): ArrayBuffer | undefined {
        return this.#read("sha1");
    }
    /** The SHA-256 checksum. */
    get sha256(): ArrayBuffer | undefined {
        return this.#read("sha256");
    }
    /** The SHA-384 checksum. */
    get sha384(): ArrayBuffer | undefined {
        return this.#read("sha384");
    }
    /** The SHA-512 checksum. */
    get sha512(): ArrayBuffer | undefined {
        return this.#read("sha512");
    }

    /** Encode stored checksums as hexadecimal strings. */
    toJSON(): Partial<Record<ChecksumAlgorithm, string>> {
        return { ...this.#digests };
    }

    /** Decode a stored checksum. */
    #read(algorithm: ChecksumAlgorithm): ArrayBuffer | undefined {
        const value = this.#digests[algorithm];

        return value === undefined ? undefined : Uint8Array.fromHex(value).buffer;
    }
}
