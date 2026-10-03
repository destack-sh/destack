import type { Package } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { found } from "@destack/schema";

/** Keep a build of a package in memory with its outputs and their files. */
export async function memoryBuild(
    built: Package,
    outputs: PackageManifest["outputs"],
    files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
): Promise<BuildReader> {
    // describe the output files
    const bytes = new Map(files);
    const described = await Promise.all(
        [...files].map(([path, contents]) =>
            PackageFile.describe(path, "text/javascript", contents),
        ),
    );

    // describe the file, dependency and source map lists, and an empty graph
    const index = async (path: string, value: unknown) => {
        const contents = new TextEncoder().encode(JSON.stringify(value));
        bytes.set(path, contents);

        return PackageFile.describe(path, "application/json", contents);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: built,
        language: "typescript",
        lists: {
            dependencies: await index("manifest/dependencies.json", {}),
            files: await index("manifest/files.json", described),
            sourceMaps: await index("manifest/sourceMaps.json", []),
            graph: await index("manifest/graph.json", { modules: {} }),
        },
        outputs,
    });

    return new BuildReader(manifest, async (path) => found(bytes, path));
}
