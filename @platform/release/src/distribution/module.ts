import { readFile } from "node:fs/promises";
import { dirname, extname } from "node:path";
import { PackageLocator, transformModule } from "@destack/package/transform";
import { transformAssets } from "./asset.ts";

/** The packages whose metadata carries the distribution version. */
const VERSIONED_PACKAGES: ReadonlySet<string> = new Set([
    "@destack/cli",
    "@destack/daemon",
    "@destack/desktop",
]);

/** The Bun loader of each source extension, TypeScript for every other. */
const LOADERS: Readonly<Record<string, Bun.Loader>> = { ".tsx": "tsx", ".jsx": "jsx" };

/** Compose package metadata and asset references within Bun's single source load. */
export function modulePlugin(root: string, assets: Set<string>, version?: string): Bun.BunPlugin {
    // share package lookup results across modules in this compilation
    const packages = new PackageLocator();

    return {
        name: "destack-executable-module",
        setup(build) {
            build.onLoad({ filter: /\.[cm]?[jt]sx?$/u }, async ({ path }) => {
                // preserve directory references before injecting declaration metadata
                const source = await readFile(path, "utf8");
                const transformed = await transformAssets(source, path, root, assets);
                const owner = await packages.find(path);

                // stamp the distribution version into the versioned packages' metadata
                let contents = transformed;
                if (owner !== undefined) {
                    const isVersioned =
                        version !== undefined &&
                        VERSIONED_PACKAGES.has(owner.metadata.package.name);
                    const metadata = isVersioned
                        ? { ...owner.metadata, package: { ...owner.metadata.package, version } }
                        : owner.metadata;
                    const result = transformModule(
                        transformed ?? source,
                        path,
                        { ...owner, metadata },
                        packages,
                    );
                    contents = result?.code ?? transformed;
                }
                if (contents === undefined) {
                    return;
                }

                return {
                    contents,
                    loader: LOADERS[extname(path)] ?? "ts",
                    resolveDir: dirname(path),
                };
            });
        },
    };
}
