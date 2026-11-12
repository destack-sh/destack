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

/** The bytes a write gathers before writing them: 1 MiB, a few writes per pack of small files. */
const WRITE_BYTES = 1024 * 1024;

/** The files a blob store keeps its blobs in, and the randomness naming its writes. */
export type BlobEnvironment = Pick<Environment<"files">, "files" | "random">;

/** How a blob store syncs its writes, as SQLite's `synchronous`: fully, or off for a cache. */
export type BlobSynchronous = "full" | "off";

/** A bucket's blobs as files: runs of a write's small blobs in packs, large blobs alone. */
export class LocalBlobStorage implements BlobStorage {
    /** The directory of pack files. */
    readonly directory: string;
    /** The files the blobs live in. */
    readonly #files: FileSystem;
    /** The randomness naming each write's temporary file and each pack of several blobs. */
    readonly #random: Random;
    /** Whether each write syncs its file and directory, off for a cache its digests verify. */
    readonly #synchronous: BlobSynchronous;

    /** Keep blobs in a directory of a file system. */
    private constructor(
        directory: string,
        environment: BlobEnvironment,
        options: { readonly synchronous: BlobSynchronous },
    ) {
        // keep the directory, its files, and how writes sync
        this.directory = directory;
        this.#files = environment.files;
        this.#random = environment.random;
        this.#synchronous = options.synchronous;
    }

    /** Open a directory of blobs with a sync, creating it durably. */
    static async open(
        directory: string,
        environment: BlobEnvironment,
        options: { readonly synchronous: BlobSynchronous },
    ): Promise<LocalBlobStorage> {
        await createDurably(directory, environment.files);

        return new LocalBlobStorage(directory, environment, options);
    }

    /** Build the path of a pack's file, refusing anything but a pack key. */
    path(pack: string): string {
        if (!BlobPack.isKey(pack)) {
            throw BucketError.of(
                "ARGUMENT_INVALID",
                `pack key '${pack}' is invalid: it is not a pack key`,
            );
        }

        return join(this.directory, pack);
    }

    /** Read a range of a pack's bytes from its file, each chunk read at its exact size. */
    async *slice(pack: string, offset: number, length: number): AsyncIterable<Uint8Array> {
        // read from the offset in chunks of the range's remaining size, up to one read's bytes
        await using file = await this.#files.open(this.path(pack), { read: true });
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

    /** Copy a blob's bytes to a path, cloning the file of a blob alone, else writing its range. */
    async copy(placement: BlobPlacement, destination: string): Promise<void> {
        // clone the blocks of a blob stored alone
        if (placement.pack === placement.digest) {
            await this.#files.copyFile(this.path(placement.pack), destination);
        }
        // write the range of a blob in a pack of several
        else {
            await using file = await this.#files.open(destination, {
                write: true,
                create: true,
                truncate: true,
            });
            const writer = file.writable().getWriter();
            for await (const chunk of this.slice(
                placement.pack,
                placement.offset,
                placement.size,
            )) {
                await writer.write(chunk);
            }
            await writer.close();
        }
    }

    /** Store the bodies in order, a pack per group, syncing the directory once. */
    async write(bodies: readonly BlobBody[]): Promise<readonly BlobPlacement[]> {
        // write a pack per group, in order
        const placements: BlobPlacement[] = [];
        for (const pack of BlobPack.groups(bodies)) {
            placements.push(...(await this.#writePack(pack)));
        }

        // sync the directory once for the files renamed into it
        if (placements.length > 0 && this.#synchronous === "full") {
            await syncDirectory(this.directory, this.#files);
        }

        return placements;
    }

    /** Write bodies to a fresh temporary file, then rename it to its key unless stored. */
    async #writePack(bodies: readonly BlobBody[]): Promise<BlobPlacement[]> {
        const { path: temporary, file } = await this.#temporary();
        try {
            // write, hash and count the bodies into a temporary file this write created
            const blobs = await this.#written(file, bodies);

            // key a lone blob by its digest
            const pack = BlobPack.key(blobs, () => Uuid.v4(this.#random));

            // store the file under its key once
            if (await this.#files.exists(this.path(pack))) {
                await this.#files.remove(temporary);
            } else {
                await this.#files.rename(temporary, this.path(pack));
            }

            return BlobPack.place(pack, blobs);
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
        await this.#files.remove(this.path(pack), { force: true });
    }

    /** List the packs, each a file of one blob or several. */
    async *packs(): AsyncIterable<string> {
        for (const entry of await this.#entries()) {
            if (entry.kind === "file" && BlobPack.isKey(entry.name)) {
                yield entry.name;
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
                    if (FileSystemError.is(error, "FILE_EXISTS")) {
                        return undefined;
                    }
                    throw error;
                });
            if (file !== undefined) {
                return { path, file };
            }
        }
    }

    /** Write, hash and count bodies in order into a created file, synced unless a cache. */
    async #written(
        file: FileHandle,
        bodies: readonly BlobBody[],
    ): Promise<{ readonly digest: Digest; readonly size: number }[]> {
        // gather each body's chunks into writes of up to a write's bytes
        await using opened = file;
        const writer = opened.writable().getWriter();
        const blobs: { readonly digest: Digest; readonly size: number }[] = [];
        let gathered: Uint8Array[] = [];
        let gatheredBytes = 0;
        for (const body of bodies) {
            // hash, count and gather the body's chunks, writing them once they fill a write
            const hash = createHash("sha256");
            let size = 0;
            for await (const chunk of body.body) {
                hash.update(chunk);
                size += chunk.byteLength;
                gathered.push(chunk);
                gatheredBytes += chunk.byteLength;
                if (gatheredBytes >= WRITE_BYTES) {
                    await writer.write(Buffer.concat(gathered, gatheredBytes));
                    gathered = [];
                    gatheredBytes = 0;
                }
            }

            // refuse bytes unlike the expected digest or size
            const digest = Digest.parse(hash.digest("hex"));
            BlobPack.verify(body, digest, size);
            blobs.push({ digest, size });
        }

        // write the rest
        if (gatheredBytes > 0) {
            await writer.write(Buffer.concat(gathered, gatheredBytes));
        }
        await writer.close();
        if (this.#synchronous === "full") {
            await opened.sync();
        }

        return blobs;
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
