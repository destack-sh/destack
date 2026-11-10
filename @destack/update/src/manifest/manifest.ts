import { type Environment, FileSystemError, Pipe } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { UpdateError } from "../error/error.ts";

/** The files a checkout writes at once, as HTTP/2 multiplexes their fetches over one connection. */
const CONCURRENCY = 16;

/** The longest a blob's fetch takes, in milliseconds: 5 minutes, the 120 MB runtime at 0.4 MB/s. */
const FETCH_TIMEOUT_MILLISECONDS = 300_000;

/** The mode of an executable file. */
const EXECUTABLE_MODE = 0o755;

/** The mode of any other file. */
const FILE_MODE = 0o644;

/** A manifest as the feed signs it: its files by digest and its links. */
const ManifestDocument = schema.object({
    /** The files, each by its digest, size and mode. */
    files: schema.array(
        schema.object({
            /** The path inside the manifest's directory. */
            path: schema.string(),
            /** The SHA-256 digest of the file's bytes. */
            digest: Digest,
            /** The file's size in bytes. */
            size: schema.number().int().nonnegative(),
            /** The file's mode: executable or not. */
            mode: schema.union([schema.literal(EXECUTABLE_MODE), schema.literal(FILE_MODE)]),
        }),
    ),
    /** The symbolic links, each with its relative target. */
    links: schema.array(
        schema.object({
            /** The path inside the manifest's directory. */
            path: schema.string(),
            /** The link's target, relative to its directory. */
            target: schema.string(),
        }),
    ),
});

/** A file of a manifest. */
export type ManifestFile = schema.Infer<typeof ManifestDocument>["files"][number];

/** A symbolic link of a manifest. */
export type ManifestLink = schema.Infer<typeof ManifestDocument>["links"][number];

/** Where a checkout takes each file from, and how it reports. */
export interface CheckoutOptions {
    /** The feed's blobs, each file's gzip bytes at its digest. */
    readonly blobs: URL;
    /** The directories with earlier copies of the manifest's files, read before the feed. */
    readonly sources: readonly string[];
    /** Cancel the checkout. */
    readonly signal?: AbortSignal;
    /** Report the bytes written so far, of the manifest's total. */
    readonly onProgress?: (progress: { received: number; total: number }) => void;
}

/** A directory's files by digest and its links, as a release or a toolchain ships them. */
export class Manifest {
    /** The files, sorted by path. */
    readonly files: readonly ManifestFile[];
    /** The symbolic links, sorted by path. */
    readonly links: readonly ManifestLink[];

    /** Take a manifest's files and links, refusing a path or a link that leaves its directory. */
    constructor(files: readonly ManifestFile[], links: readonly ManifestLink[]) {
        // refuse an empty or absolute path, a drive's path or a parent segment
        for (const { path } of [...files, ...links]) {
            const segments = path.replaceAll("\\", "/").split("/");
            const isDrive = /^[A-Za-z]:/u.test(path);
            if (path === "" || isAbsolute(path) || isDrive || segments.includes("..")) {
                throw UpdateError.of("UNSAFE_MANIFEST", `unsafe manifest path: ${path}`);
            }
        }

        // refuse a link whose target resolves outside the directory
        for (const link of links) {
            const target = relative("/root", resolve("/root", dirname(link.path), link.target));
            if (isAbsolute(link.target) || target === ".." || target.startsWith(`..${sep}`)) {
                throw UpdateError.of(
                    "UNSAFE_MANIFEST",
                    `manifest link escapes its directory: ${link.path}`,
                );
            }
        }

        // sort both by path, which the encoding and its digest rest on
        this.files = files.toSorted((left, right) => compare(left.path, right.path));
        this.links = links.toSorted((left, right) => compare(left.path, right.path));
    }

    /** Parse a manifest's signed encoding, refusing an unsafe path. */
    static parse(text: string): Manifest {
        const parsed = ManifestDocument.safeParse(JSON.parse(text));
        if (!parsed.success) {
            throw UpdateError.of("UNSAFE_MANIFEST", `invalid manifest: ${parsed.error.message}`);
        }

        return new Manifest(parsed.data.files, parsed.data.links);
    }

    /** Read a directory's files and links into a manifest. */
    static async read(directory: string, environment: Environment<"files">): Promise<Manifest> {
        // digest each file and read each link below the directory
        const { files, crypto } = environment;
        const read: ManifestFile[] = [];
        const links: ManifestLink[] = [];
        for (const entry of await files.readDirectory(directory, { recursive: true })) {
            const path = join(directory, entry.name);
            if (entry.kind === "symlink") {
                links.push({ path: entry.name, target: await files.readLink(path) });
            } else if (entry.kind === "file") {
                const status = await files.stat(path);
                await using handle = await files.open(path, { read: true });
                read.push({
                    path: entry.name,
                    digest: await Digest.of(handle.readable(), crypto),
                    size: status.size,
                    mode: (status.mode & 0o111) === 0 ? FILE_MODE : EXECUTABLE_MODE,
                });
            } else if (entry.kind !== "directory") {
                throw UpdateError.of(
                    "UNSAFE_MANIFEST",
                    `unsupported manifest entry: ${entry.name}`,
                );
            }
        }

        return new Manifest(read, links);
    }

    /** Report whether another manifest lists the same files and links, whatever their modes. */
    hasSameFiles(other: Manifest): boolean {
        return this.#contents() === other.#contents();
    }

    /** Describe the files by path, digest and size, and the links, leaving the modes out. */
    #contents(): string {
        const files = this.files.map(({ path, digest, size }) => ({ path, digest, size }));

        return JSON.stringify({ files, links: this.links });
    }

    /** Take the modes another manifest records for the same files, refusing a file it lacks. */
    withModesOf(recorded: Manifest): Manifest {
        // keep each file's recorded mode, as copies between machines may drop the modes
        const modes = new Map(recorded.files.map((file) => [file.path, file.mode]));
        const files = this.files.map((file) => {
            const mode = modes.get(file.path);
            if (mode === undefined) {
                throw UpdateError.of(
                    "METADATA_INVALID",
                    `the manifest records no mode of ${file.path}`,
                );
            }

            return { ...file, mode };
        });

        return new Manifest(files, this.links);
    }

    /** Encode the manifest as the feed signs it, one way for one manifest. */
    encode(): string {
        return JSON.stringify({ files: this.files, links: this.links });
    }

    /** Check the manifest out into a directory: copy what the sources have, fetch the rest. */
    async checkout(
        directory: string,
        options: CheckoutOptions,
        environment: Environment<"files">,
    ): Promise<void> {
        // write the links, then the files a few at a time
        const { files } = environment;
        for (const link of this.links) {
            await files.mkdir(dirname(join(directory, link.path)), { recursive: true });
            await files.symlink(link.target, join(directory, link.path));
        }
        const total = this.files.reduce((sum, file) => sum + file.size, 0);
        const progress = { received: 0, total };
        const pending = [...this.files];
        const writer = async (): Promise<void> => {
            for (let file = pending.shift(); file !== undefined; file = pending.shift()) {
                options.signal?.throwIfAborted();
                await Manifest.#writeFile(directory, file, options, environment);
                progress.received += file.size;
                options.onProgress?.({ ...progress });
            }
        };

        // wait for every writer before failing, so none writes on into a removed directory
        const written = await Promise.allSettled(Array.from({ length: CONCURRENCY }, writer));
        const failed = written.find((result) => result.status === "rejected");
        if (failed !== undefined) {
            throw failed.reason;
        }
        options.signal?.throwIfAborted();
    }

    /** Write one file: a source's copy of the same digest, else the feed's blob, verified. */
    static async #writeFile(
        directory: string,
        file: ManifestFile,
        options: CheckoutOptions,
        environment: Environment<"files">,
    ): Promise<void> {
        // write the first source's copy with the file's size and digest
        const { files, network, clock } = environment;
        const path = join(directory, file.path);
        await files.mkdir(dirname(path), { recursive: true });
        for (const source of options.sources) {
            const status = await files.lstat(join(source, file.path)).catch((error: unknown) => {
                if (!FileSystemError.is(error, "NOT_FOUND")) {
                    throw error;
                }

                return undefined;
            });
            if (status?.kind === "file" && status.size === file.size) {
                await using copy = await files.open(join(source, file.path), { read: true });
                const digest = await Manifest.#write(path, copy.readable(), file, environment);
                if (digest === file.digest) {
                    return;
                }
                await files.remove(path);
            }
        }

        // fetch the blob within its timeout, refusing a failed request
        const signal = AbortSignal.any([
            clock.timeout(FETCH_TIMEOUT_MILLISECONDS),
            ...(options.signal === undefined ? [] : [options.signal]),
        ]);
        const response = await network.fetch(
            new Request(new URL(file.digest, options.blobs).href, { signal }),
        );
        if (!response.ok || response.body === null) {
            await response.body?.cancel();
            throw UpdateError.of(
                "DOWNLOAD_FAILED",
                `blob request failed: HTTP ${response.status} for ${file.path}`,
            );
        }

        // write the decompressed blob, refusing bytes of another digest
        const decompressed = Pipe.through(
            response.body,
            environment.compression.decompress("gzip"),
        );
        if ((await Manifest.#write(path, decompressed, file, environment)) !== file.digest) {
            throw UpdateError.of("VERIFICATION_FAILED", `blob digest mismatch: ${file.path}`);
        }
    }

    /** Write a file's bytes up to its size while digesting them, answering their digest. */
    static async #write(
        path: string,
        bytes: AsyncIterable<Uint8Array>,
        file: ManifestFile,
        environment: Environment<"files">,
    ): Promise<Digest> {
        // write one branch of the bytes and digest the other
        const [written, digested] = Pipe.through(bytes, limit(file)).tee();
        await using handle = await environment.files.open(path, {
            write: true,
            createNew: true,
            mode: file.mode,
        });
        const [, digest] = await Promise.all([
            Pipe.to(written, handle.writable()),
            Digest.of(digested, environment.crypto),
        ]);

        return digest;
    }
}

/** Pass a blob's bytes on, refusing more than its file's size or fewer at the end. */
function limit(file: ManifestFile): TransformStream<Uint8Array, Uint8Array> {
    let received = 0;

    return new TransformStream({
        transform(chunk, controller) {
            received += chunk.byteLength;
            if (received > file.size) {
                throw UpdateError.of("VERIFICATION_FAILED", `blob exceeds its size: ${file.path}`);
            }
            controller.enqueue(chunk);
        },
        flush() {
            if (received !== file.size) {
                throw UpdateError.of("VERIFICATION_FAILED", `blob is truncated: ${file.path}`);
            }
        },
    });
}

/** Compare two paths by their code units, one order on every platform. */
function compare(left: string, right: string): number {
    return left < right ? -1 : left > right ? 1 : 0;
}
