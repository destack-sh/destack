import { Digest, schema } from "@destack/schema";
import { PackageFile, PackagePath } from "../file/file.ts";
import { DependencyResolution } from "../definition/dependency.ts";
import { SourceMapReference } from "../source/map.ts";
import { Module, Root } from "../graph/module.ts";
import { Declaration } from "../graph/declaration.ts";
import { PackageError } from "../error/index.ts";
import type { DeclarationName, PackageId } from "../definition/package.ts";
import { PackageLocation, PackageManifest } from "./manifest.ts";

/** The path of a build's root manifest beside its files, as stores and running workloads keep it. */
export const MANIFEST_PATH = "manifest.json";

/** A readable package distribution supplied by local or remote storage. */
export interface PackageDistribution {
    /** The root manifest. */
    readonly manifest: PackageManifest;
    /** Verified access to serialized descriptions. */
    readonly reader: BuildReader;
    /** Open a distributed file without retaining its complete contents in memory. */
    open(path: string, signal?: AbortSignal): Promise<ReadableStream<Uint8Array>>;
}

/** Read selected manifest descriptions through a caller-supplied file transport. */
export class BuildReader {
    /** Manifest describing the stored package. */
    readonly manifest: PackageManifest;
    /** Read one package file by its relative path. */
    readonly load: (path: string) => Promise<Uint8Array<ArrayBuffer>>;
    /** The graph file of each module, read once. */
    #modules: Promise<Module[]> | undefined;

    /** Bind a manifest to local, registry or browser file access. */
    constructor(
        manifest: PackageManifest,
        load: (path: string) => Promise<Uint8Array<ArrayBuffer>>,
    ) {
        this.manifest = manifest;
        this.load = load;
    }

    /** Read exact compiler dependency resolutions. */
    dependencies() {
        return this.read(
            this.manifest.lists.dependencies,
            schema.record(schema.string(), DependencyResolution),
        );
    }

    /** Read the source, executable and asset list. */
    files() {
        return this.read(this.manifest.lists.files, schema.array(PackageFile));
    }

    /** Read generated-file and source-map associations. */
    sourceMaps() {
        return this.read(this.manifest.lists.sourceMaps, schema.array(SourceMapReference));
    }

    /** List the files the manifest names directly. */
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
        return this.read(this.manifest.lists.graph, Root);
    }

    /** Read one module's graph file by its digest. */
    async module(digest: Digest): Promise<Module> {
        // verify the content address before parsing the file
        const path = `graph/${digest}.json`;
        const bytes = await this.load(path);
        if ((await Digest.of(bytes)) !== digest) {
            throw new PackageError("INVALID_FILE", `file digest mismatch: ${path}`);
        }

        return Module.parse(decode(bytes));
    }

    /** Read every module's graph file once. */
    modules(): Promise<Module[]> {
        this.#modules ??= this.graph().then((root) =>
            Promise.all(Object.values(root.modules).map((digest) => this.module(digest))),
        );

        return this.#modules;
    }

    /** Read the package's declarations from its graph, without the members they derive. */
    async declarations(): Promise<Declaration[]> {
        const modules = await this.modules();

        return modules
            .flatMap((module) => module.declarations)
            .filter((declaration) => !Declaration.isMember(declaration));
    }

    /** Read the declarations of one kind a package's constructors make, parsing each description. */
    async declared<Item extends schema.Schema>(
        owner: PackageId,
        kind: DeclarationName,
        item: Item,
    ): Promise<Declared<schema.Output<Item>>[]> {
        const declarations = await this.declarations();

        return declarations
            .filter((declaration) => declaration.package === owner && declaration.kind === kind)
            .map((declaration) => ({
                ...declaration,
                description: item.parse(declaration.description),
            }));
    }

    /** Verify a selected file before decoding its declared description type. */
    async read<Definition extends schema.Schema>(
        file: PackageFile,
        definition: Definition,
    ): Promise<schema.Output<Definition>> {
        // verify the exact bytes before parsing external JSON
        const bytes = await this.load(file.path);
        await PackageFile.verify(file, bytes);

        return definition.parse(decode(bytes));
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
        const bytes = await BuildReader.#fetch(new URL(MANIFEST_PATH, base), fetch, signal);
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

        return new BuildReader(manifest, async (path) => {
            const encoded = PackagePath.parse(path).split("/").map(encodeURIComponent).join("/");

            return BuildReader.#fetch(new URL(`files/${encoded}`, base), fetch, signal);
        });
    }

    /** Read one complete package file and reject HTTP failures before decoding it. */
    static async #fetch(
        url: URL,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal: AbortSignal | undefined,
    ): Promise<Uint8Array<ArrayBuffer>> {
        const response = await fetch(url, {
            redirect: "error",
            ...(signal === undefined ? {} : { signal }),
        });
        if (!response.ok) {
            await response.body?.cancel();
            throw new PackageError("INVALID_FILE", `package read failed: HTTP ${response.status}`);
        }

        return new Uint8Array(await response.arrayBuffer());
    }
}

/** Decode a JSON file's verified bytes. */
function decode(bytes: Uint8Array<ArrayBuffer>): unknown {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
}

/** A declaration a build holds, with its description parsed. */
export type Declared<Description> = Omit<Declaration, "description"> & {
    /** The description, parsed by its reader. */
    readonly description: Description;
};
