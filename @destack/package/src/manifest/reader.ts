import type { Crypto, FileSystem } from "@destack/environment";
import { present, schema } from "@destack/schema";
import { PackageFile, PackagePath } from "../file/file.ts";
import { DependencyResolution } from "../definition/dependency.ts";
import type { PackageRelease } from "../definition/package.ts";
import { SourceMapReference } from "../source/map.ts";
import { Declaration } from "../graph/declaration.ts";
import { PackageError } from "../error/index.ts";
import type { DeclarationName, PackageId } from "../definition/package.ts";
import { PackageLocation, PackageManifest } from "./manifest.ts";

/** The path of a build's root manifest beside its files, in stores and running workloads. */
export const MANIFEST_PATH = "manifest.json";

/** A build's manifest and files, read through a file transport. */
export class BuildReader {
    /** Manifest describing the stored package. */
    readonly manifest: PackageManifest;
    /** Stream one package file by its relative path, without keeping it whole in memory. */
    readonly stream: (path: string, signal?: AbortSignal) => Promise<ReadableStream<Uint8Array>>;
    /** The dependency resolutions, read once. */
    readonly #dependencies = new Once<Readonly<Record<string, DependencyResolution>>>();
    /** The file list, read once. */
    readonly #files = new Once<readonly PackageFile[]>();
    /** The source map list, read once. */
    readonly #sourceMaps = new Once<readonly SourceMapReference[]>();
    /** The declarations, read once. */
    readonly #declarations = new Once<readonly Declaration[]>();
    /** Each kind's declarations parsed once, by the parsing schema, then kind package and kind. */
    readonly #declared = new WeakMap<schema.Schema, Map<string, Promise<readonly unknown[]>>>();
    /** The listed files by path, read once. */
    readonly #listed = new Once<ReadonlyMap<string, PackageFile>>();
    /** The crypto verifying the files' digests. */
    readonly #crypto: Crypto;

    /** Bind a manifest to local, registry or browser file access, verifying with the crypto. */
    constructor(manifest: PackageManifest, stream: BuildReader["stream"], crypto: Crypto) {
        this.manifest = manifest;
        this.stream = stream;
        this.#crypto = crypto;
    }

    /** Read one distributed file whole by its relative path and verify its exact bytes. */
    async load(path: string, signal?: AbortSignal): Promise<Uint8Array<ArrayBuffer>> {
        // read at most the listed size, refusing a longer body before buffering it
        const file = await this.file(path);
        const bytes = new Uint8Array(file.size);
        let size = 0;
        for await (const chunk of await this.stream(path, signal)) {
            if (size + chunk.byteLength > file.size) {
                throw PackageError.of("FILE_INVALID", `file size mismatch: ${path}`);
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
            throw PackageError.of("FILE_INVALID", `package file not listed: ${path}`);
        }

        return file;
    }

    /** Read exact compiler dependency resolutions. */
    dependencies(): Promise<Readonly<Record<string, DependencyResolution>>> {
        return this.#dependencies.get(() =>
            this.read(
                this.manifest.lists.dependencies,
                schema.record(schema.string(), DependencyResolution),
            ),
        );
    }

    /** Read the Destack package releases the build resolved: its dependency closure's. */
    async releases(): Promise<PackageRelease[]> {
        return Object.values(await this.dependencies()).flatMap((dependency) =>
            dependency.kind === "destack"
                ? [{ package: dependency.package, manifest: dependency.manifest }]
                : [],
        );
    }

    /** Read the source, executable and asset list. */
    files(): Promise<readonly PackageFile[]> {
        return this.#files.get(() =>
            this.read(this.manifest.lists.files, schema.array(PackageFile)),
        );
    }

    /** Read generated-file and source-map associations. */
    sourceMaps(): Promise<readonly SourceMapReference[]> {
        return this.#sourceMaps.get(() =>
            this.read(this.manifest.lists.sourceMaps, schema.array(SourceMapReference)),
        );
    }

    /** List the files the manifest lists directly. */
    references(): PackageFile[] {
        return Object.values(this.manifest.lists);
    }

    /** Collect all distributed file records for complete downloads and verification. */
    async distributed(): Promise<PackageFile[]> {
        return [...this.references(), ...(await this.files())];
    }

    /** Read the package's declarations, without the members they derive. */
    async declarations(): Promise<Declaration[]> {
        const listed = await this.#declarations.get(() =>
            this.read(this.manifest.lists.declarations, schema.array(Declaration)),
        );

        return listed.filter((declaration) => !Declaration.isMember(declaration));
    }

    /** Read one kind's declarations from a package's constructors, parsing each description. */
    async declared<Item extends schema.Schema>(
        kindPackageId: PackageId,
        kind: DeclarationName,
        item: Item,
    ): Promise<Declared<schema.Output<Item>>[]> {
        // parse a kind's declarations once per schema, as a build never changes
        const key = `${kindPackageId} ${kind}`;
        const parsed = this.#declared.get(item) ?? new Map<string, Promise<readonly unknown[]>>();
        this.#declared.set(item, parsed);
        let declared = parsed.get(key);
        if (declared === undefined) {
            declared = this.declarations().then((declarations) =>
                declarations
                    .filter(
                        (declaration) =>
                            declaration.kindPackageId === kindPackageId &&
                            declaration.kind === kind,
                    )
                    .map((declaration) => ({
                        ...declaration,
                        description: item.parse(declaration.description),
                    })),
            );
            parsed.set(key, declared);
            declared.catch(() => parsed.delete(key));
        }

        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- the memo keys each parse by the schema that produced it
        return [...((await declared) as readonly Declared<schema.Output<Item>>[])];
    }

    /** Read a listed JSON file and parse it with a schema. */
    async read<Definition extends schema.Schema>(
        file: PackageFile,
        definition: Definition,
    ): Promise<schema.Output<Definition>> {
        return definition.parse(decode(await this.load(file.path)));
    }

    /** Open the build a directory keeps in files: its manifest at the root and its files below. */
    static async openDirectory(
        directory: string,
        files: FileSystem,
        crypto: Crypto,
    ): Promise<BuildReader> {
        // read the manifest, then stream each file it lists from the directory
        const bytes = await files.readFile(`${directory}/${MANIFEST_PATH}`);
        const manifest = PackageManifest.parse(decode(bytes));

        return new BuildReader(
            manifest,
            (path, signal) => streamFile(`${directory}/${PackagePath.parse(path)}`, files, signal),
            crypto,
        );
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
        const base = new URL(location.url);
        if (
            !["https:", "http:"].includes(base.protocol) ||
            base.username ||
            base.password ||
            base.search ||
            base.hash
        ) {
            throw PackageError.of("FILE_INVALID", "invalid package URL");
        }
        if (!base.pathname.endsWith("/")) {
            base.pathname += "/";
        }

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
            throw PackageError.of("FILE_INVALID", `package read failed: HTTP ${response.status}`);
        }

        return response;
    }
}

/** Stream a file's bytes, closing it at its end, on cancel, a failure or once a signal aborts. */
async function streamFile(
    file: string,
    files: FileSystem,
    signal: AbortSignal | undefined,
): Promise<ReadableStream<Uint8Array>> {
    // open the file and read its chunks
    signal?.throwIfAborted();
    const handle = await files.open(file, { read: true });
    const reader = handle.readable().getReader();

    // close the file once, whichever of its end, a failure, a cancel or the abort comes first
    let closing: Promise<void> | undefined;
    const close = () => (closing ??= handle.close());
    signal?.addEventListener("abort", () => void close(), { once: true });

    return new ReadableStream<Uint8Array>({
        pull: async (controller) => {
            try {
                // pass the next chunk on, refusing a read after the abort
                signal?.throwIfAborted();
                const next = await reader.read();

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
            await reader.cancel();
            await close();
        },
    });
}

/** Decode a JSON file's verified bytes. */
function decode(bytes: Uint8Array<ArrayBuffer>): unknown {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
}

/** A declaration of a build, with its description parsed. */
export type Declared<Description> = Omit<Declaration, "description"> & {
    /** The description, parsed by its reader. */
    readonly description: Description;
};

/** A value read on first use and read again after a failure. */
export class Once<Value> {
    /** The current read, if any. */
    #read: Promise<Value> | undefined;

    /** Return the value, reading it on first use and after a failure. */
    get(read: () => Promise<Value>): Promise<Value> {
        this.#read ??= read().catch((error: unknown) => {
            this.#read = undefined;
            throw error;
        });

        return this.#read;
    }
}
