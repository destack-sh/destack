import { readFile } from "node:fs/promises";
import { dirname } from "node:path";
import { ModulePackages, transformModule } from "@destack/package/transform";
import { transformAssets } from "./asset.ts";

/** Compose package metadata and asset references within Bun's single source load. */
export function modulePlugin(root: string, assets: Set<string>, version?: string): Bun.BunPlugin {
    // share package lookup results across modules in this compilation
    const packages = new ModulePackages();
    const names = new Set(["@destack/cli", "@destack/daemon", "@destack/desktop"]);

    return {
        name: "destack-executable-module",
        setup(build) {
            build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, async ({ path }) => {
                // preserve directory references before injecting declaration metadata
                const source = await readFile(path, "utf8");
                const transformed = await transformAssets(source, path, root, assets);
                const owner = await packages.find(path);
                const metadata = owner && {
                    ...owner.metadata,
                    package:
                        version && names.has(owner.metadata.package.name)
                            ? { ...owner.metadata.package, version }
                            : owner.metadata.package,
                };
                const result = metadata && transformModule(transformed ?? source, path, metadata);
                if (!result && transformed === undefined) {
                    return;
                }

                return {
                    contents: result?.code ?? transformed!,
                    loader: path.endsWith("tsx") ? "tsx" : path.endsWith("jsx") ? "jsx" : "ts",
                    resolveDir: dirname(path),
                };
            });
        },
    };
}
