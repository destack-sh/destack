import { schema } from "@destack/schema";
import { verifyFile, type PackageFile } from "../file/file.ts";
import { DependencyResolution } from "../definition/dependency.ts";
import { SourceMapReference } from "../source/map.ts";
import { ModuleDescription } from "../code/module.ts";
import { PackageError } from "../error/index.ts";
import type { DeclarationName, PackageId } from "../definition/package.ts";
import { DeclarationDescription } from "../inspect/declaration.ts";
import { ManifestFile, type PackageManifest } from "./manifest.ts";

/** A readable package distribution supplied by local or remote storage. */
export interface PackageDistribution {
    /** The root manifest. */
    readonly manifest: PackageManifest;
    /** Verified access to serialized descriptions. */
    readonly reader: PackageReader;
    /** Open a distributed file without retaining its complete contents in memory. */
    open(path: string, signal?: AbortSignal): Promise<ReadableStream<Uint8Array>>;
}

/** Read selected manifest descriptions through a caller-supplied file transport. */
export class PackageReader {
    /** Manifest describing the stored package. */
    readonly manifest: PackageManifest;
    /** Read one package file by its relative path. */
    readonly load: (path: string) => Promise<Uint8Array<ArrayBuffer>>;

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
            this.manifest.dependencies,
            schema.record(schema.string(), DependencyResolution),
        );
    }

    /** Read the source, executable and asset inventory. */
    files() {
        return this.read(this.manifest.files, schema.array(ManifestFile));
    }

    /** Read generated-file and source-map associations. */
    sourceMaps() {
        return this.read(this.manifest.sourceMaps, schema.array(SourceMapReference));
    }

    /** Collect all distributed file records for complete downloads and verification. */
    async inventory(): Promise<PackageFile[]> {
        // follow authenticated indexes without loading individual descriptions
        const files = await this.files();
        const manifest = this.manifest;

        return [
            manifest.dependencies,
            manifest.files,
            manifest.sourceMaps,
            ...Object.values(manifest.descriptions).map((collection) => collection.file),
            ...files.flatMap((file) =>
                (file.descriptions ?? []).map((description) => description.file),
            ),
            ...files,
        ];
    }

    /** Read one module description selected from the file inventory. */
    module(file: PackageFile) {
        return this.read(file, ModuleDescription);
    }

    /** Read and validate one domain collection. */
    domain<Definition extends schema.Schema>(
        name: string,
        definition: Definition,
    ): Promise<schema.Output<Definition>> {
        const collection = this.manifest.descriptions[name];
        if (!collection) {
            throw new PackageError("INVALID_FILE", `unknown description collection: ${name}`);
        }

        return this.read(collection.file, definition);
    }

    /** Read the descriptions of one kind of declaration a package's constructors make. */
    async declared<Item extends schema.Schema>(
        owner: PackageId,
        kind: DeclarationName,
        item: Item,
    ): Promise<schema.Output<Item>[]> {
        // read the owner's collection of each version the build holds
        const collections = Object.values(this.manifest.descriptions).filter(
            (collection) => collection.package.id === owner,
        );
        const declarations = await Promise.all(
            collections.map((collection) =>
                this.read(collection.file, schema.array(DeclarationDescription)),
            ),
        );

        return declarations
            .flat()
            .filter((declaration) => declaration.kind === kind)
            .map((declaration) => item.parse(declaration.description));
    }

    /** Verify a selected file before decoding its declared description type. */
    async read<Definition extends schema.Schema>(
        file: PackageFile,
        definition: Definition,
    ): Promise<schema.Output<Definition>> {
        // verify the exact bytes before parsing external JSON
        const bytes = await this.load(file.path);
        await verifyFile(file, bytes);
        const document = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));

        return definition.parse(document);
    }
}
