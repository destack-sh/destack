import type { Package } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { found } from "@destack/schema";

/** Keep a build of a package in memory that ships no files, as tests start its workloads. */
export async function emptyBuild(owner: Package): Promise<BuildReader> {
    // keep each list as an empty JSON file
    const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
    const list = async (path: string, value: unknown) => {
        const contents = new TextEncoder().encode(JSON.stringify(value));
        bytes.set(path, contents);

        return PackageFile.describe(path, "application/json", contents);
    };

    // describe the build of no outputs over them
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: owner,
        language: "typescript",
        lists: {
            dependencies: await list("manifest/dependencies.json", {}),
            files: await list("manifest/files.json", []),
            sourceMaps: await list("manifest/sourceMaps.json", []),
            graph: await list("manifest/graph.json", { modules: {} }),
        },
        outputs: {},
    });

    return new BuildReader(manifest, async (path) => found(bytes, path));
}
