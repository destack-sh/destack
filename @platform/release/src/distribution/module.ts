import type { FileSystem } from "@destack/environment";
import { dirname, extname } from "node:path";
import { PackageLocator } from "@destack/package/locator";
import { DECLARED_MODULE, DECLARED_PASSTHROUGH, transformModule } from "@destack/package/transform";
import { transformAssets } from "./asset.ts";

/** The packages whose metadata has the distribution version. */
const VERSIONED_PACKAGES: ReadonlySet<string> = new Set([
    "@destack/cli",
    "@destack/machine",
    "@destack/desktop",
]);

/** The namespace the declaration recorder loads from, apart from the files. */
const DECLARED_NAMESPACE = "destack-declared";

/** The Bun loader of each source extension, TypeScript for every other. */
const LOADERS: Readonly<Record<string, Bun.Loader>> = { ".tsx": "tsx", ".jsx": "jsx" };

/** Compose package metadata and asset references within Bun's single source load. */
export function modulePlugin(
    root: string,
    assets: Set<string>,
    files: FileSystem,
    version?: string,
): Bun.BunPlugin {
    // share package lookup results across modules in this compilation
    const packages = new PackageLocator(files);

    return {
        name: "destack-executable-module",
        setup(build) {
            // provide the declaration recorder as its pass-through
            build.onResolve({ filter: /^virtual:destack-declared$/u }, () => ({
                path: DECLARED_MODULE,
                namespace: DECLARED_NAMESPACE,
            }));
            build.onLoad({ filter: /.*/u, namespace: DECLARED_NAMESPACE }, () => ({
                contents: DECLARED_PASSTHROUGH,
                loader: "js",
            }));

            // stamp each source file
            build.onLoad({ filter: /\.[cm]?[jt]sx?$/u }, async ({ path }) => {
                // preserve directory references before injecting declaration metadata
                const source = await files.readTextFile(path);
                const transformed = await transformAssets(source, path, root, assets, files);
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
                    const result = await transformModule(
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
