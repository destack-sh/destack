/** A cursor over a stream's chunks, reading exact byte counts without copying within a chunk. */
export class ChunkReader implements AsyncDisposable {
    /** The chunks of the stream. */
    readonly #chunks: AsyncIterator<Uint8Array>;
    /** Make the error for a stream that ends before a read's last byte. */
    readonly #ended: () => Error;
    /** The unread bytes of the current chunk. */
    #current: Uint8Array<ArrayBuffer> = new Uint8Array(0);

    /** Read the chunks of a stream, failing with the given error when it ends early. */
    constructor(chunks: AsyncIterable<Uint8Array>, ended: () => Error) {
        this.#chunks = chunks[Symbol.asyncIterator]();
        this.#ended = ended;
    }

    /** Read exactly some bytes, as a view within one chunk, else copied across chunks. */
    async read(count: number): Promise<Uint8Array<ArrayBuffer>> {
        // view the bytes within the current chunk
        if (this.#current.byteLength >= count) {
            const bytes = this.#current.subarray(0, count);
            this.#current = this.#current.subarray(count);

            return bytes;
        }

        // copy the bytes across chunks
        const bytes = new Uint8Array(count);
        let offset = 0;
        for await (const part of this.stream(count)) {
            bytes.set(part, offset);
            offset += part.byteLength;
        }

        return bytes;
    }

    /** Stop the stream, releasing what its source holds open. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#chunks.return?.(undefined);
    }

    /** Stream exactly some bytes as views of the chunks. */
    async *stream(count: number): AsyncGenerator<Uint8Array<ArrayBuffer>> {
        for (let remaining = count; remaining > 0;) {
            // take the next chunk once the current one is read
            if (this.#current.byteLength === 0) {
                const next = await this.#chunks.next();
                if (next.done === true) {
                    throw this.#ended();
                }
                this.#current = view(next.value);
            }

            // hand out what the current chunk has of the rest
            const part = this.#current.subarray(0, remaining);
            this.#current = this.#current.subarray(part.byteLength);
            remaining -= part.byteLength;
            yield part;
        }
    }
}

/** View a chunk over its own buffer, copying only a chunk over a shared one. */
function view(chunk: Uint8Array): Uint8Array<ArrayBuffer> {
    const { buffer, byteOffset, byteLength } = chunk;

    return buffer instanceof ArrayBuffer
        ? new Uint8Array(buffer, byteOffset, byteLength)
        : new Uint8Array(chunk);
}
