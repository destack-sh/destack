import type { Clock } from "../clock/clock.ts";

/** The WebCrypto operations code runs, each settling on a worker thread outside the event loop. */
export const CRYPTO_OPERATIONS = [
    "decrypt",
    "deriveBits",
    "deriveKey",
    "digest",
    "encrypt",
    "exportKey",
    "generateKey",
    "importKey",
    "sign",
    "verify",
    "wrapKey",
    "unwrapKey",
] as const;

/** The algorithm WebCrypto hashes with, such as SHA-256. */
type DigestAlgorithm = Parameters<SubtleCrypto["digest"]>[0];

/** The bytes WebCrypto hashes at once. */
type DigestData = Parameters<SubtleCrypto["digest"]>[1];

/** The WebCrypto operations an environment runs, hashing streams as Deno's digest does. */
export type Crypto = Pick<SubtleCrypto, Exclude<(typeof CRYPTO_OPERATIONS)[number], "digest">> & {
    /** Hash bytes, or a stream of chunks, with an algorithm. */
    readonly digest: (
        algorithm: DigestAlgorithm,
        data: DigestData | AsyncIterable<Uint8Array>,
    ) => Promise<ArrayBuffer>;
};

/** Hash a stream of chunks with an algorithm. */
export type StreamDigest = (
    algorithm: DigestAlgorithm,
    chunks: AsyncIterable<Uint8Array>,
) => Promise<ArrayBuffer>;

/** WebCrypto operations. */
export const Crypto = {
    /** The system's WebCrypto operations, hashing a stream once its chunks gathered. */
    get system(): Crypto {
        return Crypto.of(crypto.subtle, gather);
    },

    /** Take a WebCrypto's operations, hashing streams with a stream digest. */
    of(subtle: SubtleCrypto, digestStream: StreamDigest): Crypto {
        return {
            decrypt: subtle.decrypt.bind(subtle),
            deriveBits: subtle.deriveBits.bind(subtle),
            deriveKey: subtle.deriveKey.bind(subtle),
            encrypt: subtle.encrypt.bind(subtle),
            exportKey: subtle.exportKey.bind(subtle),
            generateKey: subtle.generateKey.bind(subtle),
            importKey: subtle.importKey.bind(subtle),
            sign: subtle.sign.bind(subtle),
            verify: subtle.verify.bind(subtle),
            wrapKey: subtle.wrapKey.bind(subtle),
            unwrapKey: subtle.unwrapKey.bind(subtle),
            digest: (algorithm, data) =>
                Symbol.asyncIterator in data
                    ? digestStream(algorithm, data)
                    : subtle.digest(algorithm, data),
        };
    },

    /** Count each operation of a crypto on a clock as host work until it settles. */
    track<Subtle extends object>(subtle: Subtle, clock: Clock): Subtle {
        // count the listed operations, and read every other member as it is
        const operations: ReadonlySet<PropertyKey> = new Set(CRYPTO_OPERATIONS);

        return new Proxy(subtle, {
            get(target, property) {
                const member: unknown = Reflect.get(target, property);
                if (!operations.has(property) || typeof member !== "function") {
                    return member;
                }

                return (...parameters: unknown[]) => {
                    // run the operation, and leave a streamed digest untracked, as it waits on its chunks
                    const result: unknown = Reflect.apply(member, target, parameters);
                    const data = parameters[1];
                    const isStream =
                        property === "digest" &&
                        typeof data === "object" &&
                        data !== null &&
                        Symbol.asyncIterator in data;

                    return isStream ? result : clock.trackHost(Promise.resolve(result));
                };
            },
        });
    },
};

/** Hash a stream with the WebCrypto once its chunks gathered. */
async function gather(algorithm: DigestAlgorithm, chunks: AsyncIterable<Uint8Array>) {
    // gather the chunks into one buffer
    const parts: Uint8Array[] = [];
    let length = 0;
    for await (const chunk of chunks) {
        parts.push(chunk);
        length += chunk.byteLength;
    }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const part of parts) {
        bytes.set(part, offset);
        offset += part.byteLength;
    }

    return crypto.subtle.digest(algorithm, bytes);
}
