import type { Crypto } from "@destack/environment";
import { type Commit, Digest, schema } from "@destack/schema";
import type { DependencyResolution, NpmPackage, Package } from "../definition/index.ts";
import { Declaration } from "../graph/declaration.ts";
import { PackageDefinition } from "../definition/index.ts";
import { PackageError } from "../error/error.ts";
import { PackageFile } from "../file/file.ts";
import type { SourceMapReference } from "../source/map.ts";
import type { PackageManifest } from "./manifest.ts";
import { type BuildReader, MANIFEST_PATH } from "./reader.ts";

/** The path of each list a build's manifest references, inside the build. */
const MANIFEST_LISTS = {
    dependencies: "manifest/dependencies.json",
    files: "manifest/files.json",
    sourceMaps: "manifest/sourceMaps.json",
    declarations: "manifest/declarations.json",
} as const;

/** The package manifests a build retains, which define its exports, runtimes and stamps. */
const DEFINITION_FILES: ReadonlySet<string> = new Set(["package.json", "destack.json"]);

/** The media types of a build's files by extension (IANA), octet streams otherwise. */
const MEDIA_TYPES: Readonly<Record<string, string>> = {
    json: "application/json",
    map: "application/json",
    js: "text/javascript",
    html: "text/html",
    css: "text/css",
    wasm: "application/wasm",
};

/** Where a build writer keeps file bytes: a build directory, or memory. */
export interface BuildStorage {
    /** Keep a file's bytes at a package path. */
    readonly write: (path: string, bytes: Uint8Array<ArrayBuffer>) => Promise<void>;
}

/** What a build's manifest records beside its files. */
export interface BuildContent {
    /** The package being built: a Destack package, or an npm release built as a unit. */
    readonly package: Package | NpmPackage;
    /** The commit the build compiled, absent for a working tree with uncommitted changes. */
    readonly commit?: Commit;
    /** The named outputs, none when absent. */
    readonly outputs?: PackageManifest["outputs"];
    /** The exact dependencies the compiler used, by name and version, none when absent. */
    readonly dependencies?: Readonly<Record<string, DependencyResolution>>;
    /** The source maps of generated files, none when absent. */
    readonly sourceMaps?: readonly SourceMapReference[];
    /** The declarations the build's evaluation described, none when absent. */
    readonly declarations?: readonly Declaration[];
    /** Whether the package extends its dependents' builds, whose interface then has every file. */
    readonly isExtension?: boolean;
}

/** Write a build for `BuildReader`: its files, then its lists, then its manifest. */
export class BuildWriter {
    /** Where the bytes go. */
    readonly #storage: BuildStorage;
    /** The crypto digesting the written files. */
    readonly #crypto: Crypto;
    /** The written files, by package path. */
    readonly #files = new Map<string, PackageFile>();
    /** The writes of files' bytes under way, by package path. */
    readonly #writing = new Map<string, Promise<void>>();

    /** Write a build into storage, digesting its files with the crypto. */
    constructor(storage: BuildStorage, crypto: Crypto) {
        this.#storage = storage;
        this.#crypto = crypto;
    }

    /** Write a build again from another's files, lists and declarations under new outputs. */
    static async relink(
        source: BuildReader,
        storage: BuildStorage,
        crypto: Crypto,
        outputs: Required<BuildContent>["outputs"],
    ): Promise<PackageManifest> {
        // read the source's lists, and whether its package extends its dependents' builds
        const read = source.manifest;
        const [files, dependencies, sourceMaps, declarations] = await Promise.all([
            source.files(),
            source.dependencies(),
            source.sourceMaps(),
            source.read(read.lists.declarations, schema.array(Declaration)),
        ]);
        const definition = files.some((file) => file.path === "destack.json")
            ? PackageDefinition.parse(
                  JSON.parse(new TextDecoder().decode(await source.load("destack.json"))),
              )
            : undefined;

        // record its files as they are, reading none, then finish under the changes
        const writer = new BuildWriter(storage, crypto);
        for (const file of files) {
            writer.add(file);
        }

        return await writer.finish({
            package: read.package,
            ...(read.commit === undefined ? {} : { commit: read.commit }),
            outputs,
            dependencies,
            sourceMaps,
            declarations,
            isExtension: definition?.build !== undefined,
        });
    }

    /** Name a build file's media type by its extension. */
    static mediaType(path: string): string {
        return MEDIA_TYPES[path.slice(path.lastIndexOf(".") + 1)] ?? "application/octet-stream";
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<PackageFile> {
        // accept an equal file written before
        const file = await PackageFile.describe(
            path,
            BuildWriter.mediaType(path),
            bytes,
            this.#crypto,
        );
        if (this.#isWritten(file)) {
            await this.#writing.get(path);

            return file;
        }

        // record the file while its bytes are kept, so a concurrent equal write waits for them
        this.#files.set(path, file);
        const writing = this.#storage.write(path, bytes);
        this.#writing.set(path, writing);
        try {
            await writing;
        } catch (error) {
            // forget a file whose bytes were not kept
            this.#files.delete(path);
            throw error;
        } finally {
            this.#writing.delete(path);
        }

        return file;
    }

    /** Record a file in storage, such as compiler output, refusing other bytes at its path. */
    add(file: PackageFile): void {
        if (!this.#isWritten(file)) {
            this.#files.set(file.path, file);
        }
    }

    /** Read the description of a written file. */
    file(path: string): PackageFile {
        const file = this.#files.get(path);
        if (file === undefined) {
            throw PackageError.of("FILE_INVALID", `unwritten build file: ${path}`);
        }

        return file;
    }

    /** Write the lists, then the manifest listing them, and return it. */
    async finish(contents: BuildContent): Promise<PackageManifest> {
        // write the lists, with the list of every file before them
        const files = [...this.#files.values()].toSorted((left, right) =>
            left.path < right.path ? -1 : Number(left.path > right.path),
        );
        const lists = {
            dependencies: await this.#writeJson(
                MANIFEST_LISTS.dependencies,
                contents.dependencies ?? {},
            ),
            files: await this.#writeJson(MANIFEST_LISTS.files, files),
            sourceMaps: await this.#writeJson(MANIFEST_LISTS.sourceMaps, contents.sourceMaps ?? []),
            declarations: await this.#writeJson(
                MANIFEST_LISTS.declarations,
                contents.declarations ?? [],
            ),
        };

        // digest what dependents' compilations read of the build
        const outputs = contents.outputs ?? {};
        const interfaceDigest = await Digest.json(
            {
                package: contents.package,
                manifests: files.filter((file) => DEFINITION_FILES.has(file.path)),
                exports: Object.fromEntries(
                    Object.entries(outputs).map(([name, output]) => [
                        name,
                        {
                            runtime: output.runtime,
                            exports: Object.keys(output.exports).toSorted(),
                        },
                    ]),
                ),
                declarations: lists.declarations.digest,
                files: contents.isExtension === true ? lists.files.digest : null,
            },
            this.#crypto,
        );

        // write the manifest last, which makes the build readable
        const manifest: PackageManifest = {
            formatVersion: 1,
            package: contents.package,
            language: "typescript",
            ...(contents.commit === undefined ? {} : { commit: contents.commit }),
            lists,
            interface: interfaceDigest,
            outputs,
        };
        await this.#storage.write(
            MANIFEST_PATH,
            new TextEncoder().encode(JSON.stringify(manifest)),
        );

        return manifest;
    }

    /** Report whether a file was written with these bytes, refusing other bytes at its path. */
    #isWritten(file: PackageFile): boolean {
        const previous = this.#files.get(file.path);
        if (previous !== undefined && previous.digest !== file.digest) {
            throw PackageError.of("FILE_INVALID", `conflicting build file: ${file.path}`);
        }

        return previous !== undefined;
    }

    /** Write a description as readable JSON. */
    #writeJson(path: string, value: unknown): Promise<PackageFile> {
        return this.write(path, new TextEncoder().encode(`${JSON.stringify(value, null, 4)}\n`));
    }
}
