import { createHash } from "node:crypto";
import { mkdir, open, rename, stat, unlink } from "node:fs/promises";
import { join } from "node:path";
import { DatabaseError } from "../error/error.ts";
import { Digest } from "@destack/schema";
import type { BlobStore } from "./blob.ts";

/** The bytes one read of a blob file takes: 1 MiB, a few milliseconds of disk at 500 MB/s. */
const READ_BYTES = 1024 * 1024;

/** A directory of blob files, each named by its digest, whose owner deletes them. */
export class LocalBlobStore implements Omit<BlobStore, "hold" | "retire"> {
    /** The directory. */
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
            throw new DatabaseError("INVALID_BLOB", `not a blob digest: ${digest}`);
        }

        return join(this.directory, digest);
    }

    /** List the digests the directory lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        const found = await Promise.all(
            digests.map((digest) =>
                stat(this.path(digest)).then(
                    () => true,
                    (error: NodeJS.ErrnoException) => {
                        if (error.code !== "ENOENT") {
                            throw error;
                        }

                        return false;
                    },
                ),
            ),
        );

        return digests.filter((_, index) => found[index] !== true);
    }

    /** Read a blob's bytes a megabyte at a time. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        await using file = await open(this.path(digest), "r");
        for (let position = 0; ;) {
            const buffer = new Uint8Array(READ_BYTES);
            const { bytesRead } = await file.read(buffer, 0, READ_BYTES, position);
            if (bytesRead === 0) {
                return;
            }
            yield buffer.subarray(0, bytesRead);
            position += bytesRead;
        }
    }

    /** Write bytes to a temporary file, then keep it under its digest, sharing a blob kept already. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // write and hash the bytes into a temporary file
        const temporary = join(this.directory, `.${crypto.randomUUID()}`);
        const hash = createHash("sha256");
        try {
            {
                await using file = await open(temporary, "wx", 0o600);
                for await (const chunk of body) {
                    await file.write(chunk);
                    hash.update(chunk);
                }
                await file.sync();
            }

            // refuse bytes of another digest, and keep the file under its digest once
            const digest = hash.digest("hex");
            if (expected !== undefined && digest !== expected) {
                throw new DatabaseError("INVALID_BLOB", `blob ${expected} read as ${digest}`);
            }
            const [missing] = await this.missing([digest]);
            if (missing === undefined) {
                await unlink(temporary);
            } else {
                await rename(temporary, this.path(digest));
                await sync(this.directory);
            }

            return digest;
        } catch (error) {
            await unlink(temporary).catch((cleanup: NodeJS.ErrnoException) => {
                if (cleanup.code !== "ENOENT") {
                    throw new AggregateError([error, cleanup], "blob write and cleanup failed", {
                        cause: error,
                    });
                }
            });
            throw error;
        }
    }

    /** Keep the blobs the directory lacks among some digests, read from another store. */
    async fetch(digests: readonly Digest[], source: Pick<BlobStore, "read">): Promise<void> {
        for (const digest of await this.missing([...new Set(digests)])) {
            await this.write(source.read(digest), digest);
        }
    }

    /** Delete a blob's file. */
    async delete(digest: Digest): Promise<void> {
        await unlink(this.path(digest));
    }
}

/** Flush a directory's entries, so a renamed file survives a crash. */
async function sync(directory: string): Promise<void> {
    // request write access for Windows FlushFileBuffers
    const mode = process.platform === "win32" ? "r+" : "r";
    await using handle = await open(directory, mode);
    await handle.sync();
}
