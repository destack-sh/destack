/** The formats a compression stream writes and reads, as `CompressionStream` names them. */
export type CompressionFormat = "gzip" | "deflate" | "deflate-raw";

/** A stream pair that compresses or decompresses the bytes written to it. */
export type CompressionPair = ReadableWritablePair<
    Uint8Array<ArrayBuffer>,
    Uint8Array<ArrayBuffer>
>;

/** The compression an environment runs: gzip and deflate streams, as `CompressionStream` does. */
export interface Compression {
    /** Compress the bytes written in a format. */
    readonly compress: (format: CompressionFormat) => CompressionPair;
    /** Decompress the bytes written in a format. */
    readonly decompress: (format: CompressionFormat) => CompressionPair;
}

/** Compression sources. */
export const Compression = {
    /** The platform's compression streams, which run on a native pool. */
    system: {
        compress: (format) => new CompressionStream(format),
        decompress: (format) => new DecompressionStream(format),
    } satisfies Compression,
};
