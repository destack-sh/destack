import {
    deflateRawSync,
    deflateSync,
    gunzipSync,
    gzipSync,
    inflateRawSync,
    inflateSync,
} from "node:zlib";
import type { Compression, CompressionFormat, CompressionPair } from "./compression.ts";

/** The synchronous compression of each format. */
const COMPRESS: Readonly<Record<CompressionFormat, (bytes: Uint8Array) => Uint8Array>> = {
    gzip: gzipSync,
    deflate: deflateSync,
    "deflate-raw": deflateRawSync,
};

/** The synchronous decompression of each format. */
const DECOMPRESS: Readonly<Record<CompressionFormat, (bytes: Uint8Array) => Uint8Array>> = {
    gzip: gunzipSync,
    deflate: inflateSync,
    "deflate-raw": inflateRawSync,
};

/** A world's compression, transforming each whole input at once and synchronously. */
export const SimulatedCompression: Compression = {
    compress: (format) => buffered(COMPRESS[format]),
    decompress: (format) => buffered(DECOMPRESS[format]),
};

/** Collect a stream's chunks, and transform them at once when it ends. */
function buffered(transform: (bytes: Uint8Array) => Uint8Array): CompressionPair {
    const chunks: Uint8Array[] = [];

    return new TransformStream<Uint8Array<ArrayBuffer>, Uint8Array<ArrayBuffer>>({
        transform: (chunk) => {
            chunks.push(chunk);
        },
        flush: (controller) => {
            // join the chunks, then transform them at once
            const input = new Uint8Array(chunks.reduce((size, chunk) => size + chunk.length, 0));
            let offset = 0;
            for (const chunk of chunks) {
                input.set(chunk, offset);
                offset += chunk.length;
            }
            controller.enqueue(new Uint8Array(transform(input)));
        },
    });
}
