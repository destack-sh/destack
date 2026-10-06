import { Digest, found } from "@destack/schema";
import type { Package } from "../definition/package.ts";
import { Declaration } from "../graph/declaration.ts";
import type { Module } from "../graph/module.ts";
import { Moniker } from "../graph/moniker.ts";
import type { PackageManifest } from "../manifest/manifest.ts";
import { BuildReader, type PackageDistribution } from "../manifest/reader.ts";
import { type BuildContents, BuildWriter } from "../manifest/writer.ts";

/** The module declarations live in unless they name one, the package's entry. */
const ENTRY_MODULE = "src/index.ts";

/** A value a build declares, as its kind describes it, in its module, the entry when absent. */
export interface DeclaredValue {
    /** The declaration kind. */
    readonly kind: string;
    /** The package defining the kind. */
    readonly package: string;
    /** The declaration's name, also its symbol's. */
    readonly name: string;
    /** The kind's description, JSON its kind parses. */
    readonly description: Readonly<Record<string, unknown>>;
    /** The declaring module. */
    readonly module?: string;
}

/** The parts of a declaration a declared value gives. */
const Declared = Declaration.omit({ moniker: true, symbol: true });

/** A build kept in memory: written through a `BuildWriter` and read back through a `BuildReader` that verifies every digest. */
export class MemoryBuild implements PackageDistribution {
    /** The build's manifest. */
    readonly manifest: PackageManifest;
    /** The verified reader of the build's descriptions. */
    readonly reader: BuildReader;
    /** The build's files by package path. */
    readonly #files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;

    /** Keep a written build's manifest and files. */
    private constructor(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ) {
        this.manifest = manifest;
        this.reader = new BuildReader(manifest, async (path) => found(files, path));
        this.#files = files;
    }

    /** Write a build into memory from its files by package path and what its manifest records. */
    static async write(
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        contents: BuildContents,
    ): Promise<MemoryBuild> {
        // write the files, then the format naming them
        const stored = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter({
            write: async (path, bytes) => {
                stored.set(path, bytes);
            },
        });
        for (const [path, bytes] of files) {
            await writer.write(path, bytes);
        }

        return new MemoryBuild(await writer.finish(contents), stored);
    }

    /** Write a build declaring values, each module holding them written as a file with its graph module, beside further files and contents. */
    static async declaring(
        owner: Package,
        declared: readonly DeclaredValue[],
        contents: Omit<BuildContents, "package" | "graph"> & {
            /** Further files by package path. */
            readonly files?: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
        } = {},
    ): Promise<MemoryBuild> {
        // write each declaring module, describing its declarations at their symbols
        const { files: further, ...rest } = contents;
        const files = new Map(further);
        const graph: Module[] = [];
        for (const [path, values] of Map.groupBy(
            declared,
            (value) => value.module ?? ENTRY_MODULE,
        )) {
            const source = new TextEncoder().encode("export {};\n");
            files.set(path, source);
            graph.push({
                path,
                digest: await Digest.of(source),
                imports: [],
                exports: [],
                symbols: [],
                declarations: values.map(({ module: _module, ...value }) =>
                    Declaration.at(
                        Moniker.of({ packageId: owner.id, module: path, name: value.name }),
                        Declared.parse(value),
                    ),
                ),
                edges: [],
            });
        }

        return await MemoryBuild.write(files, { ...rest, package: owner, graph });
    }

    /** Open a file of the build. */
    async open(path: string): Promise<ReadableStream<Uint8Array>> {
        return new Blob([found(this.#files, path)]).stream();
    }
}
