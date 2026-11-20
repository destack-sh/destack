import type { FileSystem } from "@destack/environment";
import { dirname, extname } from "node:path";
import { PackageLocator } from "@destack/package/locator";
import { Version } from "@destack/schema";
import { DECLARED_MODULE, DECLARED_PASSTHROUGH, transformModule } from "@destack/package/transform";

/** The namespace the declaration recorder loads from, apart from the files. */
const DECLARED_NAMESPACE = "destack-declared";

/** The Bun loader of each source extension, TypeScript for every other. */
const LOADERS: Readonly<Record<string, Bun.Loader>> = { ".tsx": "tsx", ".jsx": "jsx" };

/** Stamp package metadata into each source within Bun's single source load. */
export function modulePlugin(files: FileSystem, version?: string): Bun.BunPlugin {
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

            // stamp each source file of a package
            build.onLoad({ filter: /\.[cm]?[jt]sx?$/u }, async ({ path }) => {
                // leave files outside Destack packages to the bundler
                const owner = await packages.find(path);
                if (owner === undefined) {
                    return;
                }

                // stamp the distribution version into the metadata of each package no release gave one
                const isVersioned =
                    version !== undefined && owner.metadata.package.version === Version.development;
                const metadata = isVersioned
                    ? { ...owner.metadata, package: { ...owner.metadata.package, version } }
                    : owner.metadata;
                const source = await files.readTextFile(path);
                const contents = await transformModule(
                    source,
                    path,
                    { ...owner, metadata },
                    packages,
                );
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
