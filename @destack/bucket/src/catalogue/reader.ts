import { aligned } from "@destack/schema";
import { BucketError } from "../error/index.ts";
import type { segment } from "./stack/index.ts";
import type { CustomerKey } from "./encryption.ts";
import type { BlobStore } from "./store.ts";

/** One blob of a stored file, its nonce and its length. */
export type Segment = Pick<typeof segment.$inferSelect, "blob" | "nonce" | "size">;

/** A ranged stream across a file's content segments that closes them before reporting completion. */
export class ContentReader {
    /** The selected content stream. */
    readonly stream: ReadableStream<Uint8Array>;
    /** The blobs of the segments. */
    readonly #blobs: BlobStore;
    /** The file's content segments in order. */
    readonly #segments: Segment[];
    /** Release the bucket's references to the segments. */
    readonly #release: () => void;
    /** The customer key decrypting the segments, absent for plain content. */
    readonly #key: CustomerKey | undefined;
    /** The segment with the next byte. */
    #index = 0;
    /** The next byte to read within the current segment. */
    #offset: number;
    /** The remaining selected bytes. */
    #remaining: number;
    /** The chunks of the current segment's selected range. */
    #chunks: AsyncIterator<Uint8Array> | undefined;
    /** The cipher of the current segment, absent for plain content. */
    #cipher: ReturnType<CustomerKey["cipher"]> | undefined;
    /** The pending or completed closure. */
    #closing: Promise<void> | undefined;
    /** Whether the consumer cancelled this stream. */
    #isCancelled = false;

    /** Read one range of retained segments, releasing them once. */
    constructor(
        blobs: BlobStore,
        segments: Segment[],
        offset: number,
        length: number,
        release: () => void,
        key?: CustomerKey,
    ) {
        // retain the segments and skip to the one with the first selected byte
        this.#blobs = blobs;
        this.#segments = segments;
        this.#release = release;
        this.#key = key;
        this.#offset = offset;
        this.#remaining = length;
        for (const segment of segments) {
            if (this.#offset < segment.size) {
                break;
            }
            this.#offset -= segment.size;
            this.#index++;
        }

        // open the pull stream
        this.stream = new ReadableStream({
            pull: (controller) => this.#pull(controller),
            cancel: () => this.#cancel(),
        });
    }

    /** Read one chunk and close before the end-of-stream notification. */
    async #pull(controller: ReadableStreamDefaultController<Uint8Array>): Promise<void> {
        try {
            // pass on the next chunk of the selected range
            if (this.#remaining !== 0) {
                await this.#read(controller);
            }

            // close once the range ends
            if (this.#remaining === 0) {
                await this.#close();
                if (!this.#isCancelled) {
                    controller.close();
                }
            }
        } catch (error) {
            await this.#fail(controller, error);
        }
    }

    /** Read, decrypt and pass on the next chunk of the current segment's selected range. */
    async #read(controller: ReadableStreamDefaultController<Uint8Array>): Promise<void> {
        // read the current segment's selected range
        const segment = aligned(this.#segments, this.#index);
        this.#chunks ??= this.#open(segment);

        // refuse content shorter than the catalogue records
        const next = await this.#chunks.next();
        if (this.#isCancelled) {
            return;
        }
        if (next.done === true) {
            throw new BucketError(
                "INCOMPLETE_BODY",
                "stored file is shorter than its recorded size",
            );
        }

        // decrypt the chunk at its position and pass it on
        const read = next.value;
        const bytes = this.#cipher === undefined ? read : await this.#cipher(read, this.#offset);
        if (this.#isCancelled) {
            return;
        }
        this.#offset += read.byteLength;
        this.#remaining -= read.byteLength;
        controller.enqueue(bytes);

        // close a finished segment and continue with the next one
        if (this.#offset === segment.size) {
            const chunks = this.#chunks;
            this.#chunks = undefined;
            this.#index++;
            this.#offset = 0;
            await chunks.return?.();
        }
    }

    /** Open the chunks of a segment's selected range, with the cipher decrypting them. */
    #open(segment: Segment): AsyncIterator<Uint8Array> {
        // slice the selected bytes of the segment
        const length = Math.min(this.#remaining, segment.size - this.#offset);
        this.#cipher = segment.nonce === null ? undefined : this.#key?.cipher(segment.nonce);
        const range = this.#blobs.slice(segment.blob, this.#offset, length);

        return range[Symbol.asyncIterator]();
    }

    /** Close after a failed read and report the failure, with a failed closure beside it. */
    async #fail(
        controller: ReadableStreamDefaultController<Uint8Array>,
        error: unknown,
    ): Promise<void> {
        // close, keeping a closure failure beside the read failure
        let failure = error;
        try {
            await this.#close();
        } catch (cleanup) {
            if (cleanup !== error) {
                failure = new AggregateError([error, cleanup], "file read and closure failed");
            }
        }

        // report the failure unless the consumer cancelled
        if (!this.#isCancelled) {
            controller.error(failure);
        }
    }

    /** Wait for the open segment to close before completing cancellation. */
    async #cancel(): Promise<void> {
        this.#isCancelled = true;
        await this.#close();
    }

    /** Close the open segment once and release the retained segments. */
    #close(): Promise<void> {
        // detach the open segment so a pending advance does not close it twice
        const chunks = this.#chunks;
        this.#chunks = undefined;
        this.#closing ??= Promise.resolve(chunks?.return?.())
            .then(() => {})
            .finally(this.#release);

        return this.#closing;
    }
}
