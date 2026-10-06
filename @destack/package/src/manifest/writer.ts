import type { Commit, JsonValue } from "@destack/schema";
import type { DependencyResolution, Package } from "../definition/index.ts";
import { PackageError } from "../error/error.ts";
import { PackageFile } from "../file/file.ts";
import { Module, type Root } from "../graph/module.ts";
import type { SourceMapReference } from "../source/map.ts";
import type { PackageManifest } from "./manifest.ts";
import { MANIFEST_PATH } from "./reader.ts";

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
    write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<void>;
}

/** What a build's manifest records beside its files. */
export interface BuildContents {
    /** The package being built. */
    readonly package: Package;
    /** The commit the build compiled, absent for a working tree with uncommitted changes. */
    readonly commit?: Commit;
    /** The named outputs, none when absent. */
    readonly outputs?: PackageManifest["outputs"];
    /** The exact dependencies the compiler used, by name and version, none when absent. */
    readonly dependencies?: Readonly<Record<string, DependencyResolution>>;
    /** The source maps of generated files, none when absent. */
    readonly sourceMaps?: readonly SourceMapReference[];
    /** The graph file of each module, each describing one of the build's files, none when absent. */
    readonly graph?: readonly Module[];
    /** The upgrade from the previous release, qualified by the package defining its format. */
    readonly upgrade?: { readonly package: Package; readonly description: JsonValue };
}

/** Write a build in the format a `BuildReader` reads: files by digest, a graph file per module, the lists, and the manifest last. */
export class BuildWriter {
    /** Where the bytes go. */
    readonly #storage: BuildStorage;
    /** The written files, by package path. */
    readonly #files = new Map<string, PackageFile>();

    /** Write a build into storage. */
    constructor(storage: BuildStorage) {
        this.#storage = storage;
    }

    /** Name a build file's media type by its extension. */
    static mediaType(path: string): string {
        return MEDIA_TYPES[path.slice(path.lastIndexOf(".") + 1)] ?? "application/octet-stream";
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<PackageFile> {
        // accept an equal file written before
        const file = await PackageFile.describe(path, BuildWriter.mediaType(path), bytes);
        if (this.#isWritten(file)) {
            return file;
        }

        // keep the bytes before recording the file
        await this.#storage.write(path, bytes);
        this.#files.set(path, file);

        return file;
    }

    /** Record a file the storage already holds, such as a compiler's output, refusing different bytes at its path. */
    add(file: PackageFile): void {
        if (!this.#isWritten(file)) {
            this.#files.set(file.path, file);
        }
    }

    /** Read the description of a written file. */
    file(path: string): PackageFile {
        const file = this.#files.get(path);
        if (file === undefined) {
            throw new PackageError("INVALID_FILE", `unwritten build file: ${path}`);
        }

        return file;
    }

    /** Write the graph files, the lists and the upgrade, then the manifest naming them, returning the manifest. */
    async finish(contents: BuildContents): Promise<PackageManifest> {
        // write each module's graph file by its digest, refusing a module of no written file
        const root: Root = { modules: {} };
        for (const module of contents.graph ?? []) {
            if (!this.#files.has(module.path)) {
                throw new PackageError(
                    "INVALID_FILE",
                    `graph module of no build file: ${module.path}`,
                );
            }
            const encoded = await Module.file(module);
            await this.write(`graph/${encoded.digest}.json`, encoded.bytes);
            root.modules[module.path] = encoded.digest;
        }

        // write the lists, the file list naming every file before them
        const files = [...this.#files.values()].toSorted((left, right) =>
            left.path < right.path ? -1 : Number(left.path > right.path),
        );
        const lists = {
            dependencies: await this.#writeJson(
                "manifest/dependencies.json",
                contents.dependencies ?? {},
            ),
            files: await this.#writeJson("manifest/files.json", files),
            sourceMaps: await this.#writeJson(
                "manifest/sourceMaps.json",
                contents.sourceMaps ?? [],
            ),
            graph: await this.#writeJson("manifest/graph.json", root),
        };

        // write the upgrade, then the manifest last, which makes the build readable
        const { upgrade } = contents;
        const manifest: PackageManifest = {
            formatVersion: 1,
            package: contents.package,
            language: "typescript",
            ...(contents.commit === undefined ? {} : { commit: contents.commit }),
            lists,
            ...(upgrade === undefined
                ? {}
                : {
                      upgrade: {
                          package: upgrade.package,
                          file: await this.#writeJson("manifest/upgrade.json", upgrade.description),
                      },
                  }),
            outputs: contents.outputs ?? {},
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
            throw new PackageError("INVALID_FILE", `conflicting build file: ${file.path}`);
        }

        return previous !== undefined;
    }

    /** Write a description as readable JSON. */
    #writeJson(path: string, value: unknown): Promise<PackageFile> {
        return this.write(path, new TextEncoder().encode(`${JSON.stringify(value, null, 4)}\n`));
    }
}
