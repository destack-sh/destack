import {
    type Compression,
    type Crypto,
    type FileHandle,
    type FileSystem,
    Once,
} from "@destack/environment";
import { Tarball, TarballEntry } from "../archive/tarball.ts";
import { Digest, present, schema } from "@destack/schema";
import { PackageFile, PackagePath } from "../file/file.ts";
import { SourceMapReference } from "../source/map.ts";
import { Declaration } from "../graph/declaration.ts";
import { PackageError } from "../error/index.ts";
import { PackageLocation, PackageManifest } from "./manifest.ts";
import type { Runtime } from "../runtime/runtime.ts";

/** The path of a build's root manifest beside its files, in stores and running workloads. */
export const MANIFEST_PATH = "manifest.json";

/** The status of a server unable to answer now, Service Unavailable (RFC 9110 15.6.4). */
const SERVICE_UNAVAILABLE = 503;

/** A build's manifest and files, read through a file transport. */
export class BuildReader {
    /** Manifest describing the stored package. */
    readonly manifest: PackageManifest;
    /** Stream one package file by its relative path, without keeping it whole in memory. */
    readonly stream: (path: string, signal?: AbortSignal) => Promise<ReadableStream<Uint8Array>>;
    /** The file list, read once. */
    readonly #files = new Once<readonly PackageFile[]>();
    /** The source map list, read once. */
    readonly #sourceMaps = new Once<readonly SourceMapReference[]>();
    /** The declarations, read once. */
    readonly #declarations = new Once<readonly Declaration[]>();
    /** The listed files by path, read once. */
    readonly #listed = new Once<ReadonlyMap<string, PackageFile>>();
    /** The manifest's digest, the build's address, taken once. */
    readonly #digest = new Once<Digest>();
    /** The crypto verifying the files' digests. */
    readonly #crypto: Crypto;

    /** Bind a manifest to local, registry or browser file access, verifying with the crypto. */
    constructor(manifest: PackageManifest, stream: BuildReader["stream"], crypto: Crypto) {
        this.manifest = manifest;
        this.stream = stream;
        this.#crypto = crypto;
    }

    /** Take the digest of the build's manifest, its address in every store, once. */
    digest(): Promise<Digest> {
        return this.#digest.get(() =>
            Digest.of(PackageManifest.serialize(this.manifest), this.#crypto),
        );
    }

    /** Read one distributed file whole by its relative path and verify its exact bytes. */
    async load(path: string, signal?: AbortSignal): Promise<Uint8Array<ArrayBuffer>> {
        // read at most the listed size, refusing a longer body before buffering it
        const file = await this.file(path);
        const bytes = new Uint8Array(file.size);
        let size = 0;
        for await (const chunk of await this.stream(path, signal)) {
            if (size + chunk.byteLength > file.size) {
                throw PackageError.of(
                    "FILE_INVALID",
                    `file '${path}' is invalid: its size does not match`,
                );
            }
            bytes.set(chunk, size);
            size += chunk.byteLength;
        }

        // require the listed size and digest
        await PackageFile.verify(file, bytes.subarray(0, size), this.#crypto);

        return bytes;
    }

    /** Read a distributed file's listing by its path, refusing an unlisted path. */
    async file(path: string): Promise<PackageFile> {
        // find the manifest's own references first, as the file list is one of them
        const reference = this.references().find((file) => file.path === path);
        if (reference !== undefined) {
            return reference;
        }

        // find the path in the file list
        const listed = await this.#listed.get(
            async () => new Map((await this.files()).map((file) => [file.path, file])),
        );
        const file = listed.get(path);
        if (file === undefined) {
            throw PackageError.of(
                "FILE_INVALID",
                `package file '${path}' is invalid: it is not listed`,
            );
        }

        return file;
    }

    /** Read the source, executable and asset list. */
    files(): Promise<readonly PackageFile[]> {
        return this.#files.get(() => this.read<PackageFile[]>(this.manifest.lists.files));
    }

    /** Read the files an output's runtime loads, by the output's name. */
    loaded(name: string): Promise<readonly PackageFile[]> {
        const output = present(this.manifest.outputs[name], `the output ${name}`);

        return this.read<PackageFile[]>(output.files);
    }

    /** Read the source maps of an output's files, which symbolicate what its runtime loads. */
    async maps(name: string): Promise<readonly PackageFile[]> {
        const { directory } = present(this.manifest.outputs[name], `the output ${name}`);

        return (await this.files()).filter(
            (file) => file.path.startsWith(`${directory}/`) && file.path.endsWith(".map"),
        );
    }

    /** List the files runtimes run of the build: the lists, and their outputs with their maps. */
    async runFiles(runtimes: readonly Runtime[]): Promise<PackageFile[]> {
        // take the lists, then each output on the runtimes with its maps
        const names = Object.entries(this.manifest.outputs)
            .filter(([, output]) => runtimes.includes(output.runtime))
            .map(([name]) => name);
        const outputs = await Promise.all(
            names.map(async (name) => [...(await this.loaded(name)), ...(await this.maps(name))]),
        );
        const files = [...this.references(), ...outputs.flat()];

        return [...new Map(files.map((file) => [file.path, file])).values()];
    }

    /** Read generated-file and source-map associations. */
    sourceMaps(): Promise<readonly SourceMapReference[]> {
        return this.#sourceMaps.get(() =>
            this.read<SourceMapReference[]>(this.manifest.lists.sourceMaps),
        );
    }

    /** List the files the manifest lists directly: its lists and each output's. */
    references(): PackageFile[] {
        const { files, sourceMaps, declarations } = this.manifest.lists;
        const outputs = Object.values(this.manifest.outputs).map((output) => output.files);

        return [files, sourceMaps, declarations, ...outputs];
    }

    /** Collect all distributed file records for complete downloads and verification. */
    async distributed(): Promise<PackageFile[]> {
        return [...this.references(), ...(await this.files())];
    }

    /** Read the package's declarations, without the members they derive. */
    async declarations(): Promise<Declaration[]> {
        const listed = await this.#declarations.get(() =>
            this.read<Declaration[]>(this.manifest.lists.declarations),
        );

        return listed.filter((declaration) => !Declaration.isMember(declaration));
    }

    /** Refuse lists without their formats, as a store checks a build it takes in from outside. */
    async check(): Promise<void> {
        // parse each list against its format
        const { files, sourceMaps, declarations } = this.manifest.lists;
        const checked = [
            [files, schema.array(PackageFile)],
            [sourceMaps, schema.array(SourceMapReference)],
            [declarations, schema.array(Declaration)],
            ...Object.values(this.manifest.outputs).map(
                (output) => [output.files, schema.array(PackageFile)] as const,
            ),
        ] as const;
        for (const [list, format] of checked) {
            const parsed = format.safeParse(await this.read(list));
            if (!parsed.success) {
                throw PackageError.of(
                    "FILE_INVALID",
                    `list '${list.path}' is invalid: it does not have its format`,
                );
            }
        }
    }

    /** Read a listed JSON file, typed by its verified digest as the build wrote it. */
    async read<Value>(file: PackageFile): Promise<Value> {
        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- the verified digest fixes the bytes the build wrote
        return decode(await this.load(file.path)) as Value;
    }

    /** Write the build, or the files given of it, to a new directory, its manifest last. */
    async writeDirectory(
        directory: string,
        files: FileSystem,
        selected?: readonly PackageFile[],
    ): Promise<void> {
        // require a new directory so a failed write cannot damage an existing build
        await files.mkdir(directory);
        for (const file of selected ?? (await this.distributed())) {
            const destination = `${directory}/${PackagePath.parse(file.path)}`;
            await files.mkdir(destination.slice(0, destination.lastIndexOf("/")), {
                recursive: true,
            });
            await files.writeFile(destination, await this.load(file.path));
        }

        // make the build readable only after every file is written
        const manifest = PackageManifest.serialize(this.manifest);
        await files.writeFile(`${directory}/${MANIFEST_PATH}`, manifest, { createNew: true });
    }

    /** Open the build a directory keeps, reading the files it lacks from elsewhere when given. */
    static async openDirectory(
        directory: string,
        files: FileSystem,
        crypto: Crypto,
        missing?: BuildReader["stream"],
    ): Promise<BuildReader> {
        // read the manifest, then stream each file it lists from the directory, else from where the caller keeps it
        const bytes = await files.readFile(`${directory}/${MANIFEST_PATH}`);
        const manifest = PackageManifest.parse(decode(bytes));
        const stream: BuildReader["stream"] = async (path, signal) => {
            const file = `${directory}/${PackagePath.parse(path)}`;

            return missing === undefined || (await files.exists(file))
                ? streamFile(file, files, signal)
                : missing(path, signal);
        };

        return new BuildReader(manifest, stream, crypto);
    }

    /** Open an immutable remote package and verify its root manifest. */
    static async open(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        crypto: Crypto,
        signal?: AbortSignal,
    ): Promise<BuildReader> {
        // restrict relative reads to this package's HTTP endpoint
        location = PackageLocation.parse(location);
        const base = BuildReader.#base(location);

        // authenticate and verify the root before trusting its file references
        const response = await BuildReader.#request(new URL(MANIFEST_PATH, base), fetch, signal);
        const bytes = new Uint8Array(await response.arrayBuffer());
        await PackageFile.verify(
            {
                path: MANIFEST_PATH,
                digest: location.manifest,
                size: bytes.byteLength,
                mediaType: "application/json",
            },
            bytes,
            crypto,
        );
        const manifest = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        return new BuildReader(
            manifest,
            async (path, read) => {
                const encoded = PackagePath.parse(path)
                    .split("/")
                    .map(encodeURIComponent)
                    .join("/");
                const file = await BuildReader.#request(
                    new URL(`files/${encoded}`, base),
                    fetch,
                    read ?? signal,
                );

                return present(file.body, "a package file's body");
            },
            crypto,
        );
    }

    /** Read a remote build, whole or one output's, in one request for its archive. */
    static async archive(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        environment: { readonly crypto: Crypto; readonly compression: Compression },
        options: {
            /** The signal ending the request. */
            readonly signal?: AbortSignal | undefined;
            /** The output whose runtime's files and maps to take, every file when absent. */
            readonly output?: string;
        } = {},
    ): Promise<BuildReader> {
        // read the archive's entries, the manifest first, refusing one not matching its digest
        location = PackageLocation.parse(location);
        const url = new URL("archive", BuildReader.#base(location));
        if (options.output !== undefined) {
            url.searchParams.set("output", options.output);
        }
        const response = await BuildReader.#request(url, fetch, options.signal);
        const files = new Map<string, Uint8Array<ArrayBuffer>>();
        for await (const entry of Tarball.read(present(response.body, "a package's archive"))) {
            files.set(entry.path, await TarballEntry.bytes(entry));
        }
        const bytes = present(files.get(MANIFEST_PATH), "the archive's manifest");
        await PackageFile.verify(
            {
                path: MANIFEST_PATH,
                digest: location.manifest,
                size: bytes.byteLength,
                mediaType: "application/json",
            },
            bytes,
            environment.crypto,
        );

        // read each file from the archive, verified as the reader reads it
        return new BuildReader(
            PackageManifest.parse(decode(bytes)),
            async (path) =>
                new Blob([present(files.get(path), `the archived file ${path}`)]).stream(),
            environment.crypto,
        );
    }

    /** Read a remote package's endpoint, refusing anything but a plain HTTP URL. */
    static #base(location: PackageLocation): URL {
        // refuse credentials, queries, fragments and other schemes, and end the path with a slash
        const base = new URL(location.url);
        if (
            !["https:", "http:"].includes(base.protocol) ||
            base.username ||
            base.password ||
            base.search ||
            base.hash
        ) {
            throw PackageError.of(
                "FILE_INVALID",
                "package URL is invalid: it must be HTTP without credentials, query or fragment",
            );
        }
        if (!base.pathname.endsWith("/")) {
            base.pathname += "/";
        }

        return base;
    }

    /** Request one package file, rejecting HTTP failures before its body is read. */
    static async #request(
        url: URL,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal: AbortSignal | undefined,
    ): Promise<Response> {
        const response = await fetch(url, {
            redirect: "manual",
            ...(signal === undefined ? {} : { signal }),
        });
        if (!response.ok) {
            await response.body?.cancel();
            const address = `${url.origin}${url.pathname}`;
            const reason =
                response.status === SERVICE_UNAVAILABLE ? "FILE_UNAVAILABLE" : "FILE_INVALID";
            throw PackageError.of(
                reason,
                `package read failed: HTTP ${response.status} at ${address}`,
            );
        }

        return response;
    }
}

/** A file opened for streaming: its handle and the reader of its bytes. */
interface OpenedFile {
    /** The open file. */
    readonly handle: FileHandle;
    /** The reader of its bytes. */
    readonly reader: ReturnType<typeof readerOf>;
}

/** Read an open file's bytes as its stream's reader. */
function readerOf(handle: FileHandle) {
    return handle.readable().getReader();
}

/** Stream a file, opening it on the first read, so an unread stream holds no handle. */
async function streamFile(
    file: string,
    files: FileSystem,
    signal: AbortSignal | undefined,
): Promise<ReadableStream<Uint8Array>> {
    // open the file at the first read, closing it once whichever of its end, a failure, a cancel or the abort comes first
    signal?.throwIfAborted();
    let opened: Promise<OpenedFile> | undefined;
    let closing: Promise<void> | undefined;
    const open = (): Promise<OpenedFile> => {
        opened ??= files
            .open(file, { read: true })
            .then((handle): OpenedFile => ({ handle, reader: readerOf(handle) }));

        return opened;
    };
    const close = () =>
        (closing ??=
            opened === undefined ? Promise.resolve() : opened.then(({ handle }) => handle.close()));
    signal?.addEventListener("abort", () => void close(), { once: true });

    return new ReadableStream<Uint8Array>({
        pull: async (controller) => {
            try {
                // pass the next chunk on, refusing a read after the abort
                signal?.throwIfAborted();
                const next = await (await open()).reader.read();

                // close the file at its end
                if (next.done) {
                    await close();
                    controller.close();
                }
                // pass a chunk on
                else {
                    controller.enqueue(next.value);
                }
            } catch (error) {
                await close();
                throw error;
            }
        },
        cancel: async () => {
            // cancel an opened read, then close the file
            if (opened !== undefined) {
                await (await opened).reader.cancel();
            }
            await close();
        },
    });
}

/** Decode a JSON file's verified bytes. */
function decode(bytes: Uint8Array<ArrayBuffer>): unknown {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
}
