import { open, type FileHandle } from "node:fs/promises";
import { join } from "node:path";
import { StorageError } from "../error/index.ts";
import type { segment } from "./stack/index.ts";

/** Read at most 64 KiB per filesystem request. */
const READ_SIZE = 64 * 1024;

/** One content file of a stored file and its length. */
export type Segment = Pick<typeof segment.$inferSelect, "content" | "size">;

/** A ranged stream across a file's content segments that closes them before reporting completion. */
export class ContentReader {
    /** The selected content stream. */
    readonly stream: ReadableStream<Uint8Array>;
    /** The directory holding the content files. */
    readonly #directory: string;
    /** The file's content segments in order. */
    readonly #segments: Segment[];
    /** Release the bucket's references to the segments. */
    readonly #release: () => void;
    /** The segment holding the next byte. */
    #index = 0;
    /** The next byte to read within the current segment. */
    #offset: number;
    /** The remaining selected bytes. */
    #remaining: number;
    /** The open content file of the current segment. */
    #file?: FileHandle;
    /** The pending or completed closure. */
    #closing?: Promise<void>;
    /** Whether the consumer cancelled this stream. */
    #isCancelled = false;

    /** Read one selected range of retained segments and release them exactly once. */
    constructor(
        directory: string,
        segments: Segment[],
        offset: number,
        length: number,
        release: () => void,
    ) {
        // retain the segments and skip to the one holding the first selected byte
        this.#directory = directory;
        this.#segments = segments;
        this.#release = release;
        this.#offset = offset;
        this.#remaining = length;
        while (this.#index < segments.length && this.#offset >= segments[this.#index]!.size) {
            this.#offset -= segments[this.#index]!.size;
            this.#index++;
        }

        // open the pull stream
        this.stream = new ReadableStream({
            pull: (controller) => this.#pull(controller),
            cancel: () => this.#cancel(),
        });
    }

    /** Read one bounded chunk and close before the end-of-stream notification. */
    async #pull(controller: ReadableStreamDefaultController<Uint8Array>): Promise<void> {
        try {
            if (this.#remaining !== 0) {
                // open the current segment unless the consumer cancelled meanwhile
                const segment = this.#segments[this.#index]!;
                if (this.#file === undefined) {
                    const file = await open(join(this.#directory, segment.content), "r");
                    if (this.#isCancelled) {
                        await file.close();

                        return;
                    }
                    this.#file = file;
                }

                // read within the segment and the selection
                const size = Math.min(READ_SIZE, this.#remaining, segment.size - this.#offset);
                const buffer = new Uint8Array(size);
                const { bytesRead } = await this.#file.read(buffer, 0, size, this.#offset);
                if (this.#isCancelled) {
                    return;
                }
                if (bytesRead === 0) {
                    throw new StorageError(
                        "INCOMPLETE_BODY",
                        "stored file is shorter than its recorded size",
                    );
                }
                this.#offset += bytesRead;
                this.#remaining -= bytesRead;
                controller.enqueue(buffer.subarray(0, bytesRead));

                // close a finished segment and continue with the next one
                if (this.#offset === segment.size) {
                    const file = this.#file;
                    this.#file = undefined;
                    this.#index++;
                    this.#offset = 0;
                    await file.close();
                }
            }
            if (this.#remaining === 0) {
                await this.#close();
                if (!this.#isCancelled) {
                    controller.close();
                }
            }
        } catch (error) {
            let failure = error;
            try {
                await this.#close();
            } catch (cleanup) {
                if (cleanup !== error) {
                    failure = new AggregateError([error, cleanup], "file read and closure failed");
                }
            }
            if (!this.#isCancelled) {
                controller.error(failure);
            }
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
        const file = this.#file;
        this.#file = undefined;
        this.#closing ??= Promise.resolve(file?.close()).finally(this.#release);

        return this.#closing;
    }
}
