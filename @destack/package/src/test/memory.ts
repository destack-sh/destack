import { Digest, found } from "@destack/schema";
import type { Package } from "../definition/package.ts";
import { Declaration } from "../graph/declaration.ts";
import type { Module } from "../graph/module.ts";
import { Moniker } from "../graph/moniker.ts";
import { BuildReader } from "../manifest/reader.ts";
import { type BuildContents, BuildWriter } from "../manifest/writer.ts";

/** The module declarations live in unless they name one, the package's entry. */
const ENTRY_MODULE = "src/index.ts";

/** The parts of a declaration a fixture gives, its moniker and symbol filled from its module. */
const Declared = Declaration.omit({ moniker: true, symbol: true });

/** Builds in memory: written with `BuildWriter`, read with a digest-checking `BuildReader`. */
export const MemoryBuild = {
    /** Write a build into memory from its files by package path and what its manifest records. */
    async write(
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        contents: BuildContents,
    ): Promise<BuildReader> {
        // write the files, then the format listing them
        const stored = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter({
            write: async (path, bytes) => {
                stored.set(path, bytes);
            },
        });
        for (const [path, bytes] of files) {
            await writer.write(path, bytes);
        }

        // read the written files back
        return new BuildReader(await writer.finish(contents), async (path) =>
            new Blob([found(stored, path)]).stream(),
        );
    },

    /** Write a package's build declaring some declarations, each in its module. */
    async of(
        owner: Package,
        declared: readonly (Pick<Declaration, "kind" | "name"> & {
            /** The package defining the kind, parsed as an identifier. */
            readonly kindPackageId: string;
            /** The description, parsed as JSON its kind takes. */
            readonly description: Readonly<Record<string, unknown>>;
            /** The declaring module, the entry module when absent. */
            readonly module?: string;
        })[],
        contents: Omit<BuildContents, "package" | "graph"> & {
            /** Further files by package path. */
            readonly files?: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
        } = {},
    ): Promise<BuildReader> {
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
            const symbol = (name: string) =>
                Moniker.of({ packageId: owner.id, module: path, name });
            graph.push({
                path,
                digest: await Digest.of(source),
                imports: [],
                exports: values.map((value) => ({
                    name: value.name,
                    symbol: symbol(value.name),
                    isTypeOnly: false,
                })),
                symbols: [],
                declarations: values.map(({ module: _module, ...value }) =>
                    Declaration.at(symbol(value.name), Declared.parse(value)),
                ),
                edges: [],
            });
        }

        return await MemoryBuild.write(files, { ...rest, package: owner, graph });
    },
};
