import { Digest, present, schema } from "@destack/schema";
import { PackageFile, PackagePath } from "../file/file.ts";
import { DependencyResolution } from "../definition/dependency.ts";
import type { PackageRelease } from "../definition/package.ts";
import { SourceMapReference } from "../source/map.ts";
import { Module, Root } from "../graph/module.ts";
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
    /** The graph's index root, read once. */
    readonly #graph = new Once<Root>();
    /** The graph file of each module, read once. */
    readonly #modules = new Once<readonly Module[]>();
    /** The listed files by path, read once. */
    readonly #listed = new Once<ReadonlyMap<string, PackageFile>>();

    /** Bind a manifest to local, registry or browser file access. */
    constructor(manifest: PackageManifest, stream: BuildReader["stream"]) {
        this.manifest = manifest;
        this.stream = stream;
    }

    /** Read one distributed file whole by its relative path and verify its exact bytes. */
    async load(path: string, signal?: AbortSignal): Promise<Uint8Array<ArrayBuffer>> {
        // read at most the listed size, refusing a longer body before buffering it
        const file = await this.file(path);
        const bytes = new Uint8Array(file.size);
        let size = 0;
        for await (const chunk of await this.stream(path, signal)) {
            if (size + chunk.byteLength > file.size) {
                throw new PackageError("INVALID_FILE", `file size mismatch: ${path}`);
            }
            bytes.set(chunk, size);
            size += chunk.byteLength;
        }

        // require the listed size and digest
        await PackageFile.verify(file, bytes.subarray(0, size));

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
            throw new PackageError("INVALID_FILE", `package file not listed: ${path}`);
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
        // collect the lists and the upgrade
        const manifest = this.manifest;
        const upgrade = manifest.upgrade ? [manifest.upgrade.file] : [];

        return [...Object.values(manifest.lists), ...upgrade];
    }

    /** Collect all distributed file records for complete downloads and verification. */
    async distributed(): Promise<PackageFile[]> {
        return [...this.references(), ...(await this.files())];
    }

    /** Read the index root of the build's graph. */
    graph(): Promise<Root> {
        return this.#graph.get(() => this.read(this.manifest.lists.graph, Root));
    }

    /** Read one module's graph file by its digest. */
    async module(digest: Digest): Promise<Module> {
        // verify the content address before parsing the file
        const path = `graph/${digest}.json`;
        const bytes = new Uint8Array(await new Response(await this.stream(path)).arrayBuffer());
        if ((await Digest.of(bytes)) !== digest) {
            throw new PackageError("INVALID_FILE", `file digest mismatch: ${path}`);
        }

        return Module.parse(decode(bytes));
    }

    /** Read every module's graph file once. */
    modules(): Promise<readonly Module[]> {
        return this.#modules.get(async () => {
            const root = await this.graph();

            return Promise.all(Object.values(root.modules).map((digest) => this.module(digest)));
        });
    }

    /** Read the package's declarations from its graph, without the members they derive. */
    async declarations(): Promise<Declaration[]> {
        const modules = await this.modules();

        return modules
            .flatMap((module) => module.declarations)
            .filter((declaration) => !Declaration.isMember(declaration));
    }

    /** Read one kind's declarations from a package's constructors, parsing each description. */
    async declared<Item extends schema.Schema>(
        kindPackageId: PackageId,
        kind: DeclarationName,
        item: Item,
    ): Promise<Declared<schema.Output<Item>>[]> {
        const declarations = await this.declarations();

        return declarations
            .filter(
                (declaration) =>
                    declaration.kindPackageId === kindPackageId && declaration.kind === kind,
            )
            .map((declaration) => ({
                ...declaration,
                description: item.parse(declaration.description),
            }));
    }

    /** Read a listed JSON file and parse it with a schema. */
    async read<Definition extends schema.Schema>(
        file: PackageFile,
        definition: Definition,
    ): Promise<schema.Output<Definition>> {
        return definition.parse(decode(await this.load(file.path)));
    }

    /** Open an immutable remote package and verify its root manifest. */
    static async open(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
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
            throw new PackageError("INVALID_FILE", "invalid package URL");
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
        );
        const manifest = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        return new BuildReader(manifest, async (path, read) => {
            const encoded = PackagePath.parse(path).split("/").map(encodeURIComponent).join("/");
            const file = await BuildReader.#request(
                new URL(`files/${encoded}`, base),
                fetch,
                read ?? signal,
            );

            return present(file.body, "a package file's body");
        });
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
            throw new PackageError("INVALID_FILE", `package read failed: HTTP ${response.status}`);
        }

        return response;
    }
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
