import { graph } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageManifest } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { describeSetting } from "../../../src/inspect/index.ts";
import { setting } from "../../../src/object/index.ts";
import type { Setting } from "../../../src/setting/index.ts";

/** Open a release whose graph declares some settings in one module. */
export async function release(settings: readonly Setting[]): Promise<BuildReader> {
    // declare each setting at its own symbol of the module, as the build records it
    const declarations = settings.map((declared, index): graph.Declaration => {
        const symbol = graph.Moniker.of({
            packageId: setting.package.id,
            module: "src/setting.ts",
            name: `setting${index}`,
        });

        return {
            moniker: graph.Moniker.parse(`${symbol}:setting`),
            symbol,
            kind: "setting",
            package: setting.package.id,
            name: declared.name,
            description: describeSetting(declared),
        };
    });
    const module = await graph.Module.file({
        path: "src/setting.ts",
        digest: "0".repeat(64),
        imports: [],
        exports: [],
        symbols: [],
        declarations,
        edges: [],
    });

    // keep the module's graph file under the root, and the manifest's other files empty
    const root = new TextEncoder().encode(
        JSON.stringify({ modules: { "src/setting.ts": module.digest } }),
    );
    const files = new Map([
        [`graph/${module.digest}.json`, module.bytes],
        ["manifest/graph.json", root],
    ]);
    const empty = await PackageFile.describe(
        "manifest/empty.json",
        "application/json",
        new Uint8Array(),
    );
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: setting.package,
        language: "typescript",
        lists: { dependencies: empty, files: empty, sourceMaps: empty, graph: await PackageFile.describe("manifest/graph.json", "application/json", root) },
        outputs: {},
    };

    return new BuildReader(manifest, async (path) => found(files, path));
}
