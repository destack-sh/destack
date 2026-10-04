import { graph, type Package } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageManifest } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { Feature } from "../../src/feature/index.ts";
import { describeFeature, describeMeter } from "../../src/inspect/index.ts";
import type { Meter } from "../../src/meter/index.ts";
import { customer } from "../../src/object/index.ts";

/** The module declaring the fixture's features and meters. */
const MODULE = "src/billing.ts";

/** Open a release of a package whose graph declares some features and meters in one module. */
export async function release(
    owner: Package,
    declared: readonly (Feature | Meter)[],
): Promise<BuildReader> {
    // declare each feature and meter at its own symbol of the module, as the build records it
    const declarations = declared.map((entry, index): graph.Declaration => {
        const symbol = graph.Moniker.of({
            packageId: owner.id,
            module: MODULE,
            name: `declared${index}`,
        });
        const kind = entry instanceof Feature ? "feature" : "meter";

        return {
            moniker: graph.Moniker.parse(`${symbol}:${kind}`),
            symbol,
            kind,
            package: customer.package.id,
            name: entry.name,
            description: entry instanceof Feature ? describeFeature(entry) : describeMeter(entry),
        };
    });
    const module = await graph.Module.file({
        path: MODULE,
        digest: "0".repeat(64),
        imports: [],
        exports: [],
        symbols: [],
        declarations,
        edges: [],
    });

    // keep the module's graph file under the root, and the manifest's other files empty
    const root = new TextEncoder().encode(JSON.stringify({ modules: { [MODULE]: module.digest } }));
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
        package: owner,
        language: "typescript",
        lists: {
            dependencies: empty,
            files: empty,
            sourceMaps: empty,
            graph: await PackageFile.describe("manifest/graph.json", "application/json", root),
        },
        outputs: {},
    };

    return new BuildReader(manifest, async (path) => found(files, path));
}
