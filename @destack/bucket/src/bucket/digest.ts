/** The digest algorithms a file's content digest carries. */
export const DIGEST_ALGORITHMS = ["md5", "sha1", "sha256", "sha384", "sha512"] as const;

/** A digest algorithm a file's content digest carries. */
export type DigestAlgorithm = (typeof DIGEST_ALGORITHMS)[number];

/** A file's content digests under each algorithm stored, as an HTTP Content-Digest field carries them (RFC 9530). */
export class ContentDigest {
    /** The stored hexadecimal digests. */
    readonly #digests: Partial<Record<DigestAlgorithm, string>>;

    /** Keep the stored hexadecimal digests. */
    constructor(digests: Partial<Record<DigestAlgorithm, string>> = {}) {
        this.#digests = { ...digests };
    }

    /** The MD5 digest. */
    get md5(): ArrayBuffer | undefined {
        return this.#read("md5");
    }
    /** The SHA-1 digest. */
    get sha1(): ArrayBuffer | undefined {
        return this.#read("sha1");
    }
    /** The SHA-256 digest. */
    get sha256(): ArrayBuffer | undefined {
        return this.#read("sha256");
    }
    /** The SHA-384 digest. */
    get sha384(): ArrayBuffer | undefined {
        return this.#read("sha384");
    }
    /** The SHA-512 digest. */
    get sha512(): ArrayBuffer | undefined {
        return this.#read("sha512");
    }

    /** Encode the stored digests as hexadecimal text. */
    toJSON(): Partial<Record<DigestAlgorithm, string>> {
        return { ...this.#digests };
    }

    /** Decode a stored digest. */
    #read(algorithm: DigestAlgorithm): ArrayBuffer | undefined {
        const value = this.#digests[algorithm];

        return value === undefined ? undefined : Uint8Array.fromHex(value).buffer;
    }
}
