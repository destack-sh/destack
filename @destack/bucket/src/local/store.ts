import { opendir, open, unlink } from "node:fs/promises";
import { join } from "node:path";
import type { BlobStore } from "@destack/db/blob";
import { LocalBlobStore } from "@destack/db/blob/local";
import { Digest } from "@destack/schema";
import type { ContentStore } from "../catalogue/store.ts";

/** The bytes one read of a blob file takes: 64 KiB, as a bucket's readers stream them. */
const READ_BYTES = 64 * 1024;

/** A bucket's blobs as files in a directory, each named by its digest. */
export class LocalContentStore implements ContentStore {
    /** The directory of blob files. */
    readonly #blobs: LocalBlobStore;

    /** Keep blobs in a directory store. */
    private constructor(blobs: LocalBlobStore) {
        this.#blobs = blobs;
    }

    /** Open a directory of blobs, creating it when missing. */
    static async open(directory: string): Promise<LocalContentStore> {
        return new LocalContentStore(await LocalBlobStore.open(directory));
    }

    /** List the digests the directory lacks, in the given order. */
    missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        return this.#blobs.missing(digests);
    }

    /** Read a blob's bytes. */
    read(digest: Digest): AsyncIterable<Uint8Array> {
        return this.#blobs.read(digest);
    }

    /** Read a range of a blob's bytes from its file. */
    async *slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array> {
        await using file = await open(this.#blobs.path(digest), "r");
        for (let position = offset; position < offset + length;) {
            // read the next chunk within the range
            const size = Math.min(READ_BYTES, offset + length - position);
            const buffer = new Uint8Array(size);
            const { bytesRead } = await file.read(buffer, 0, size, position);
            if (bytesRead === 0) {
                return;
            }
            yield buffer.subarray(0, bytesRead);
            position += bytesRead;
        }
    }

    /** Keep bytes under their digest. */
    write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        return this.#blobs.write(body, expected);
    }

    /** Keep the blobs the directory lacks, read from another store. */
    fetch(digests: readonly Digest[], source: Pick<BlobStore, "read">): Promise<void> {
        return this.#blobs.fetch(digests, source);
    }

    /** Delete a blob's file. */
    delete(digest: Digest): Promise<void> {
        return this.#blobs.delete(digest);
    }

    /** List the blob files. */
    async *digests(): AsyncIterable<Digest> {
        for await (const entry of await opendir(this.#blobs.directory)) {
            if (entry.isFile() && Digest.safeParse(entry.name).success) {
                yield entry.name;
            }
        }
    }

    /** Delete the temporary files of writes a terminated host left. */
    async clean(): Promise<void> {
        for await (const entry of await opendir(this.#blobs.directory)) {
            if (entry.isFile() && entry.name.startsWith(".")) {
                await unlink(join(this.#blobs.directory, entry.name));
            }
        }
    }
}
