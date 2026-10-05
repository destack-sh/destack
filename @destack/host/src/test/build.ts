import { graph, type Package } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, found } from "@destack/schema";

/** The module a memory build's declarations live in. */
const DECLARING_MODULE = "src/index.ts";

/** A declaration a memory build's graph keeps, as the declaring kind's inspector records it. */
export interface MemoryDeclaration {
    /** The declaration kind, such as resource. */
    readonly kind: string;
    /** The package defining the kind. */
    readonly definer: string;
    /** The declaration's name. */
    readonly name: string;
    /** The kind's description of the declaration. */
    readonly description: Readonly<Record<string, unknown>>;
}

/** Keep a build of a package in memory with its outputs, files and declarations. */
export async function memoryBuild(
    definition: Package,
    outputs: PackageManifest["outputs"],
    files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    declarations: readonly MemoryDeclaration[] = [],
): Promise<BuildReader> {
    // describe the output files
    const bytes = new Map(files);
    const descriptions = await Promise.all(
        [...files].map(([path, contents]) =>
            PackageFile.describe(path, "text/javascript", contents),
        ),
    );

    // keep the declarations in their module's graph file by digest, listed with the files
    const root: graph.Root = { modules: {} };
    if (declarations.length > 0) {
        const module = await declaringModule(definition, declarations);
        const path = `graph/${module.digest}.json`;
        bytes.set(path, module.bytes);
        descriptions.push(await PackageFile.describe(path, "application/json", module.bytes));
        root.modules[DECLARING_MODULE] = module.digest;
    }

    // describe the lists of files, dependencies and source maps beside the graph
    const index = async (path: string, value: unknown) => {
        const contents = new TextEncoder().encode(JSON.stringify(value));
        bytes.set(path, contents);

        return PackageFile.describe(path, "application/json", contents);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: definition,
        language: "typescript",
        lists: {
            dependencies: await index("manifest/dependencies.json", {}),
            files: await index("manifest/files.json", descriptions),
            sourceMaps: await index("manifest/sourceMaps.json", []),
            graph: await index("manifest/graph.json", root),
        },
        outputs,
    });

    return new BuildReader(manifest, async (path) => found(bytes, path));
}

/** Build the graph file of the module holding a memory build's declarations. */
async function declaringModule(
    definition: Package,
    declarations: readonly MemoryDeclaration[],
): ReturnType<typeof graph.Module.file> {
    return graph.Module.file({
        path: DECLARING_MODULE,
        digest: await Digest.of(new TextEncoder().encode(DECLARING_MODULE)),
        imports: [],
        exports: [],
        symbols: [],
        declarations: declarations.map((declaration) => {
            const symbol = graph.Moniker.of({
                packageId: definition.id,
                module: DECLARING_MODULE,
                name: declaration.name,
            });

            return graph.Declaration.parse({
                moniker: `${symbol}:${declaration.kind}`,
                symbol,
                kind: declaration.kind,
                package: declaration.definer,
                name: declaration.name,
                description: declaration.description,
            });
        }),
        edges: [],
    });
}
