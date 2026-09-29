/// <reference types="bun" />
import { readFile } from "node:fs/promises";
import { PackageLocator } from "./locator.ts";
import { transformModule } from "./transform.ts";
import { FileIndex, importVariants, requireBase, resolveVariant } from "./variant.ts";

/** Package lookups shared by every load in this process. */
const packages = new PackageLocator();

/** Inject Destack module metadata while Bun loads package sources, resolving server variants. */
export const modulePlugin: Bun.BunPlugin = {
    name: "destack-module",
    setup(build) {
        // replace relative imports with their server variants, since Bun runs server code
        const files = new FileIndex();
        const exists = (path: string) => files.has(path);
        build.onResolve({ filter: /^\./ }, ({ path, importer }) => {
            const variant = resolveVariant(path, importer, "server", exists);

            return variant === undefined ? undefined : { path: variant };
        });

        // stamp package sources, since runtime loads need contents and bundles take the rest
        const isBundling = build.config !== undefined;
        build.onLoad({ filter: /\.[cm]?tsx?$/ }, async ({ path }) => {
            // read the source and require variants to re-export their base
            const owner = await packages.find(path);
            const code = await readFile(path, "utf8");
            if (owner) {
                requireBase(code, path);
            }

            // name server variants in runtime sources
            const loaded = (!isBundling && importVariants(code, path, "server", exists)) || code;

            // pass unchanged sources on to later bundler plugins
            const result = owner && transformModule(loaded, path, owner, packages);
            if (!result && isBundling) {
                return undefined;
            }
            const loader = /\.[cm]?tsx$/.test(path) ? "tsx" : "ts";
            if (!result) {
                return { contents: loaded, loader };
            }

            // map stamped sources back to their authored lines
            const map = result.source.generateMap({ source: path });

            return { contents: `${result.code}\n//# sourceMappingURL=${map.toUrl()}`, loader };
        });
    },
};
