import type { Crypto } from "@destack/environment";
import { found } from "@destack/schema";
import type { Package } from "../definition/package.ts";
import { Declaration } from "../graph/declaration.ts";
import { Moniker } from "../graph/moniker.ts";
import { BuildReader } from "../manifest/reader.ts";
import { type BuildContent, BuildWriter } from "../manifest/writer.ts";

/** The module declarations live in unless they name one, the package's entry. */
const ENTRY_MODULE = "src/index.ts";

/** The schema address of a build's destack.json in memory, the release the fixtures declare. */
const DEFINITION_SCHEMA = "https://destack.app/schemas/2026.9.0/destack.json";

/** The parts of a declaration a fixture gives, its moniker and symbol filled from its module. */
const Declared = Declaration.omit({ moniker: true, symbol: true });

/** Builds in memory: written with `BuildWriter`, read with a digest-checking `BuildReader`. */
export const MemoryBuild = {
    /** Write a build into memory from its files by package path and what its manifest records. */
    async write(
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        contents: BuildContent,
        crypto: Crypto,
    ): Promise<BuildReader> {
        // write the files, then the format listing them
        const stored = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter(
            {
                write: async (path, bytes) => {
                    stored.set(path, bytes);
                },
            },
            crypto,
        );
        for (const [path, bytes] of files) {
            await writer.write(path, bytes);
        }

        // read the written files back
        return new BuildReader(
            await writer.finish(contents),
            async (path) => new Blob([found(stored, path)]).stream(),
            crypto,
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
        crypto: Crypto,
        contents: Omit<BuildContent, "package" | "declarations"> & {
            /** Further files by package path. */
            readonly files?: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
        } = {},
    ): Promise<BuildReader> {
        // write the package's npm and Destack definitions, as every build has, unless given
        const { files: further, ...rest } = contents;
        const files = new Map(further);
        const encoder = new TextEncoder();
        const definitions = {
            "package.json": { name: owner.name, version: owner.version, type: "module" },
            "destack.json": { $schema: DEFINITION_SCHEMA, id: owner.id, language: "typescript" },
        };
        for (const [path, definition] of Object.entries(definitions)) {
            if (!files.has(path)) {
                files.set(path, encoder.encode(JSON.stringify(definition)));
            }
        }

        // write each declaring module, describing its declarations at their symbols
        const declarations: Declaration[] = [];
        for (const [path, values] of Map.groupBy(
            declared,
            (value) => value.module ?? ENTRY_MODULE,
        )) {
            files.set(path, new TextEncoder().encode("export {};\n"));
            for (const { module: _module, ...value } of values) {
                const symbol = Moniker.of({ packageId: owner.id, module: path, name: value.name });
                declarations.push(Declaration.at(symbol, Declared.parse(value)));
            }
        }

        // record the package by its identity, name and version
        const { id, name, version } = owner;

        return await MemoryBuild.write(
            files,
            { ...rest, package: { id, name, version }, declarations },
            crypto,
        );
    },
};
