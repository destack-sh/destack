import { createHash } from "node:crypto";
import { join } from "node:path";
import {
    type DirectoryEntry,
    type Environment,
    type FileHandle,
    type FileSystem,
    FileSystemError,
    type Random,
} from "@destack/environment";
import { BlobPack, type BlobBody, type BlobPlacement, type BlobStorage } from "@destack/resource";
import { Digest, Uuid } from "@destack/schema";
import { BucketError } from "../error/index.ts";
import { createDurably, syncDirectory } from "./directory.ts";

/** The bytes one read of a blob file takes: 64 KiB, about 10 µs from the page cache. */
const READ_BYTES = 64 * 1024;

/** The files a blob store keeps its blobs in, and the randomness naming its writes. */
export type BlobEnvironment = Pick<Environment<"files">, "files" | "random">;

/** How a blob store syncs its writes, as SQLite's `synchronous`: fully, or off for a cache. */
export type BlobSynchronous = "full" | "off";

/** A bucket's blobs as files in a directory, each its own pack keyed by its digest. */
export class LocalBlobStorage implements BlobStorage {
    /** The directory of blob files. */
    readonly directory: string;
    /** The files the blobs live in. */
    readonly #files: FileSystem;
    /** The randomness naming each write's temporary file. */
    readonly #random: Random;
    /** Whether each write syncs its file and directory, off for a cache its digests verify. */
    readonly #synchronous: BlobSynchronous;

    /** Keep blobs in a directory of a file system. */
    private constructor(
        directory: string,
        environment: BlobEnvironment,
        synchronous: BlobSynchronous,
    ) {
        // keep the directory, its files, the randomness of temporary files and how writes sync
        this.directory = directory;
        this.#files = environment.files;
        this.#random = environment.random;
        this.#synchronous = synchronous;
    }

    /** Open a directory of blobs, creating it durably when missing, synced fully unless a cache. */
    static async open(
        directory: string,
        environment: BlobEnvironment,
        options: { readonly synchronous?: BlobSynchronous } = {},
    ): Promise<LocalBlobStorage> {
        await createDurably(directory, environment.files);

        return new LocalBlobStorage(directory, environment, options.synchronous ?? "full");
    }

    /** Build the path of a blob's file, refusing anything but a digest. */
    path(digest: Digest): string {
        if (!Digest.safeParse(digest).success) {
            throw BucketError.of("INVALID_ARGUMENT", `not a blob digest: ${digest}`);
        }

        return join(this.directory, digest);
    }

    /** Read a range of a pack's bytes from its file, each chunk read at its exact size. */
    async *slice(pack: string, offset: number, length: number): AsyncIterable<Uint8Array> {
        // read from the offset in chunks of the range's remaining size, up to one read's bytes
        await using file = await this.#files.open(this.path(Digest.parse(pack)), { read: true });
        await file.seek(offset);
        for (let remaining = length; remaining > 0;) {
            const buffer = new Uint8Array(Math.min(READ_BYTES, remaining));
            const bytesRead = await file.read(buffer);
            if (bytesRead === null) {
                return;
            }
            yield buffer.subarray(0, bytesRead);
            remaining -= bytesRead;
        }
    }

    /** Copy a blob's bytes to a path, cloning its blocks where the file system can. */
    copy(digest: Digest, destination: string): Promise<void> {
        return this.#files.copyFile(this.path(digest), destination);
    }

    /** Store each body as its own file under its digest, syncing the directory once. */
    async write(bodies: readonly BlobBody[]): Promise<readonly BlobPlacement[]> {
        // write each body to its file, refusing bytes unlike its expected digest or size
        const placements: BlobPlacement[] = [];
        for (const body of bodies) {
            placements.push(await this.#writeFile(body));
        }

        // sync the directory once for the files renamed into it
        if (placements.length > 0 && this.#synchronous === "full") {
            await syncDirectory(this.directory, this.#files);
        }

        return placements;
    }

    /** Write a body to a fresh temporary file, then rename it to its digest unless stored. */
    async #writeFile(body: BlobBody): Promise<BlobPlacement> {
        const { path: temporary, file } = await this.#temporary();
        try {
            // write and hash the bytes into a temporary file this write created
            const { digest, size } = await this.#written(file, body.body);
            BlobPack.verify(body, digest, size);

            // store the file under its digest once
            if (await this.#files.exists(this.path(digest))) {
                await this.#files.remove(temporary);
            } else {
                await this.#files.rename(temporary, this.path(digest));
            }

            return { digest, pack: digest, offset: 0, size };
        } catch (error) {
            await this.#removed(temporary, error);
            throw error;
        }
    }

    /** Store a file this store may consume under its digest, moving it in, and place it. */
    async move(path: string, expected: Digest): Promise<BlobPlacement> {
        // hash and count the file where it is, refusing another digest
        const bytes = await this.#files.readFile(path);
        const digest = Digest.parse(createHash("sha256").update(bytes).digest("hex"));
        const size = bytes.byteLength;
        BlobPack.verify({ digest: expected }, digest, size);

        // store the file under its digest once
        if (await this.#files.exists(this.path(digest))) {
            await this.#files.remove(path);
        } else {
            await this.#files.rename(path, this.path(digest));
            if (this.#synchronous === "full") {
                await syncDirectory(this.directory, this.#files);
            }
        }

        return { digest, pack: digest, offset: 0, size };
    }

    /** Delete a pack's file, if stored. */
    async delete(pack: string): Promise<void> {
        await this.#files.remove(this.path(Digest.parse(pack)), { force: true });
    }

    /** List the packs, each a blob's file. */
    async *packs(): AsyncIterable<string> {
        for (const entry of await this.#entries()) {
            const parsed = Digest.safeParse(entry.name);
            if (entry.kind === "file" && parsed.success) {
                yield parsed.data;
            }
        }
    }

    /** Delete the temporary files of writes a terminated host left. */
    async clean(): Promise<void> {
        for (const entry of await this.#entries()) {
            if (entry.kind === "file" && entry.name.startsWith(".")) {
                await this.#files.remove(join(this.directory, entry.name));
            }
        }
    }

    /** List the directory's entries. */
    #entries(): Promise<readonly DirectoryEntry[]> {
        return this.#files.readDirectory(this.directory);
    }

    /** Create a temporary file under a fresh name, drawing again once taken, as `mkstemp` does. */
    async #temporary(): Promise<{ readonly path: string; readonly file: FileHandle }> {
        for (;;) {
            // create the file only if no other writer did
            const path = join(this.directory, `.${Uuid.v4(this.#random)}`);
            const file = await this.#files
                .open(path, { write: true, createNew: true, mode: 0o600 })
                .catch((error: unknown) => {
                    if (FileSystemError.is(error, "ALREADY_EXISTS")) {
                        return undefined;
                    }
                    throw error;
                });
            if (file !== undefined) {
                return { path, file };
            }
        }
    }

    /** Write, hash and count a body into a created file, synced unless a cache, then close it. */
    async #written(
        file: FileHandle,
        body: AsyncIterable<Uint8Array>,
    ): Promise<{ readonly digest: Digest; readonly size: number }> {
        // write each chunk and hash and count it on the way
        await using opened = file;
        const hash = createHash("sha256");
        const writer = opened.writable().getWriter();
        let size = 0;
        for await (const chunk of body) {
            await writer.write(chunk);
            hash.update(chunk);
            size += chunk.byteLength;
        }
        await writer.close();
        if (this.#synchronous === "full") {
            await opened.sync();
        }

        return { digest: Digest.parse(hash.digest("hex")), size };
    }

    /** Remove an unfinished write's temporary file after a failure. */
    async #removed(path: string, error: unknown): Promise<void> {
        await this.#files.remove(path, { force: true }).catch((cleanup: unknown) => {
            throw new AggregateError([error, cleanup], "blob write and cleanup failed", {
                cause: error,
            });
        });
    }
}
