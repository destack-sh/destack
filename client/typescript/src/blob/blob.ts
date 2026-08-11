import type { Blob } from "../_generated/core/blob.js";
import { BlobClient, blobService } from "../_generated/blob/blob.js";
import type { Call, Connection } from "../rpc/index.js";

/** Preferred bytes in one Blob RPC item, matching the daemon's 64 KiB bound. */
const BLOB_CHUNK_BYTE_LEN = 64 * 1024;

/** One complete byte array accepted by BlobStore.put. */
export type BlobBytes = Uint8Array | readonly number[];

/** Bytes accepted by one Blob upload. */
export type BlobInput = BlobBytes | AsyncIterable<BlobBytes>;

/** One optional Blob byte range. */
export type BlobRange = {
    /** First byte to read. */
    readonly offset?: bigint;
    /** Number of bytes to read, or every remaining byte. */
    readonly byteLen?: bigint;
};

/** Immutable Blob operations over one negotiated Destack connection. */
export class BlobStore {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Complete generated Blob service client. */
    readonly client: BlobClient;

    /** Bind Blob operations to one negotiated connection. */
    constructor(connection: Connection) {
        this.connection = connection;
        this.client = new BlobClient(connection);
    }

    /** Store one complete byte array or asynchronous byte stream. */
    async put(input: BlobInput): Promise<Blob> {
        const call = this.client.put(null);
        let isInputClosed = false;

        try {
            // send one byte array or consume each supplied stream item
            if (isBlobBytes(input)) {
                await this.#send(call, input);
            } else {
                for await (const bytes of input) {
                    await this.#send(call, bytes);
                }
            }

            // publish only after the complete input stream
            await call.closeInput();
            isInputClosed = true;
            const response = await call.response();

            return response.value;
        } catch (error) {
            // cancel an upload whose input did not complete
            if (!isInputClosed) {
                try {
                    await call.cancel();
                } catch (cancelError) {
                    throw new AggregateError(
                        [error, cancelError],
                        "Blob upload and cancellation failed",
                    );
                }
            }

            throw error;
        }
    }

    /** Stream one complete Blob or byte range. */
    async *read(blob: Blob, range: BlobRange = {}): AsyncGenerator<Uint8Array> {
        const call = this.client.read({
            blob,
            offset: range.offset ?? 0n,
            byteLen: range.byteLen,
        });

        // yield every response item before requiring terminal success
        for await (const bytes of call) {
            yield blobBytes(bytes);
        }
        await call.response();
    }

    /** Read one complete Blob or byte range into memory. */
    async bytes(blob: Blob, range: BlobRange = {}): Promise<Uint8Array> {
        // validate the exact range before allocating
        const offset = range.offset ?? 0n;
        const byteLen = range.byteLen ?? blob.byteLen - offset;
        const end = offset + byteLen;
        if (offset < 0n || byteLen < 0n || end > blob.byteLen) {
            throw new RangeError(
                `Blob read range ${offset}..${end} exceeds ${blob.byteLen} bytes`,
            );
        }

        // require an exactly representable allocation length
        const length = Number(byteLen);
        if (!Number.isSafeInteger(length)) {
            throw new RangeError(`Blob read length exceeds JavaScript integer range: ${byteLen}`);
        }
        const bytes = new Uint8Array(length);
        let position = 0;

        // collect the exact streamed range into its final allocation
        for await (const chunk of this.read(blob, range)) {
            bytes.set(chunk, position);
            position += chunk.byteLength;
        }
        if (position !== bytes.byteLength) {
            throw new Error(
                `Blob read returned ${position} bytes instead of ${bytes.byteLength}`,
            );
        }

        return bytes;
    }

    /** Return whether one exact Blob is present. */
    async contains(blob: Blob): Promise<boolean> {
        const response = await this.client.contains(blob);

        return response.value;
    }

    /** Send one byte array as bounded stream items. */
    async #send(
        call: Call<Blob, Uint8Array | readonly number[], never>,
        bytes: BlobBytes,
    ): Promise<void> {
        const buffer = blobBytes(bytes);

        // retain backpressure across every bounded input item
        for (let offset = 0; offset < buffer.byteLength; offset += BLOB_CHUNK_BYTE_LEN) {
            await call.send(buffer.subarray(offset, offset + BLOB_CHUNK_BYTE_LEN));
        }
    }
}

/** Return whether one upload input is already a complete byte array. */
function isBlobBytes(input: BlobInput): input is BlobBytes {
    return input instanceof Uint8Array || Array.isArray(input);
}

/** Return one stable Uint8Array view or copy. */
function blobBytes(bytes: BlobBytes): Uint8Array {
    return bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
}

export type { Blob };
export { BlobClient, blobService };
