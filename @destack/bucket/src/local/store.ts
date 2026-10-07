import { createHash } from "node:crypto";
import { mkdir, open, opendir, rename, stat, unlink, utimes } from "node:fs/promises";
import { join } from "node:path";
import type { ContentStore } from "@destack/resource";
import { Digest } from "@destack/schema";
import type { BlobStore } from "../catalogue/store.ts";
import { BucketError } from "../error/index.ts";

/** The bytes one read of a blob file takes: 64 KiB, as a bucket's readers stream them. */
const READ_BYTES = 64 * 1024;

/** A bucket's blobs as files in a directory, each named by its digest, beside the temporary files of unfinished writes. */
export class LocalBlobStore implements BlobStore {
    /** The directory of blob files. */
    readonly directory: string;

    /** Keep blobs in a directory. */
    private constructor(directory: string) {
        this.directory = directory;
    }

    /** Open a directory of blobs, creating it when missing. */
    static async open(directory: string): Promise<LocalBlobStore> {
        await mkdir(directory, { recursive: true, mode: 0o700 });

        return new LocalBlobStore(directory);
    }

    /** Build the path of a blob's file, refusing anything but a digest. */
    path(digest: Digest): string {
        if (!Digest.safeParse(digest).success) {
            throw new BucketError("INVALID_KEY", `not a blob digest: ${digest}`);
        }

        return join(this.directory, digest);
    }

    /** List the digests the directory lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        const found = await Promise.all(digests.map((digest) => this.#has(digest)));

        return digests.filter((_, index) => found[index] !== true);
    }

    /** Read a blob's bytes. */
    read(digest: Digest): AsyncIterable<Uint8Array> {
        return this.slice(digest, 0, Number.POSITIVE_INFINITY);
    }

    /** Read a range of a blob's bytes from its file. */
    async *slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array> {
        await using file = await open(this.path(digest), "r");
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

    /** Write bytes to a temporary file, then keep it under its digest, marking a blob kept already used now. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        const temporary = join(this.directory, `.${crypto.randomUUID()}`);
        try {
            // write and hash the bytes into a temporary file
            const digest = await written(temporary, body);
            if (expected !== undefined && digest !== expected) {
                throw new BucketError("INVALID_CHECKSUM", `blob ${expected} read as ${digest}`);
            }

            // keep the file under its digest once
            if (await this.#has(digest)) {
                await unlink(temporary);
                await this.#touch(digest);
            } else {
                await rename(temporary, this.path(digest));
                await sync(this.directory);
            }

            return digest;
        } catch (error) {
            await removed(temporary, error);
            throw error;
        }
    }

    /** Keep the blobs the directory lacks among some digests, read from another store, and mark the kept ones used now. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "read">): Promise<void> {
        for (const digest of new Set(digests)) {
            // copy a missing blob
            if (!(await this.#has(digest))) {
                await this.write(source.read(digest), digest);
            }
            // mark a kept blob used
            else {
                await this.#touch(digest);
            }
        }
    }

    /** Delete a blob's file. */
    async delete(digest: Digest): Promise<void> {
        await unlink(this.path(digest));
    }

    /** List the blob files. */
    async *digests(): AsyncIterable<Digest> {
        for await (const entry of await opendir(this.directory)) {
            if (entry.isFile() && Digest.safeParse(entry.name).success) {
                yield entry.name;
            }
        }
    }

    /** Delete the temporary files of writes a terminated host left. */
    async clean(): Promise<void> {
        for await (const entry of await opendir(this.directory)) {
            if (entry.isFile() && entry.name.startsWith(".")) {
                await unlink(join(this.directory, entry.name));
            }
        }
    }

    /** Delete the blobs outside a retained set last used before a moment. */
    async sweep(retained: ReadonlySet<Digest>, before: Date): Promise<void> {
        for await (const digest of this.digests()) {
            // leave retained blobs and blobs used since the moment
            if (retained.has(digest)) {
                continue;
            }
            const { mtime } = await stat(this.path(digest));
            if (mtime < before) {
                await this.delete(digest);
            }
        }
    }

    /** Report whether the directory keeps a blob. */
    #has(digest: Digest): Promise<boolean> {
        return stat(this.path(digest)).then(
            () => true,
            (error: NodeJS.ErrnoException) => {
                if (error.code !== "ENOENT") {
                    throw error;
                }

                return false;
            },
        );
    }

    /** Mark a kept blob used now, which keeps it from a sweep's grace. */
    async #touch(digest: Digest): Promise<void> {
        const now = new Date();
        await utimes(this.path(digest), now, now);
    }
}

/** Write and hash a body into a new file, synced to disk, and return its digest. */
async function written(path: string, body: AsyncIterable<Uint8Array>): Promise<Digest> {
    // write each chunk and hash it on the way
    const hash = createHash("sha256");
    await using file = await open(path, "wx", 0o600);
    for await (const chunk of body) {
        await file.write(chunk);
        hash.update(chunk);
    }
    await file.sync();

    return hash.digest("hex");
}

/** Remove an unfinished write's temporary file after a failure, reporting both when the removal fails. */
async function removed(path: string, error: unknown): Promise<void> {
    await unlink(path).catch((cleanup: NodeJS.ErrnoException) => {
        if (cleanup.code !== "ENOENT") {
            throw new AggregateError([error, cleanup], "blob write and cleanup failed", {
                cause: error,
            });
        }
    });
}

/** Flush a directory's entries, so a renamed file survives a crash. */
async function sync(directory: string): Promise<void> {
    // request write access for Windows FlushFileBuffers
    const mode = process.platform === "win32" ? "r+" : "r";
    await using handle = await open(directory, mode);
    await handle.sync();
}
