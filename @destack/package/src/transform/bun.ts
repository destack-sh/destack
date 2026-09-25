/// <reference types="bun" />
import { readFile } from "node:fs/promises";
import { PackageLocator, transformModule } from "./transform.ts";
import { requireBase, resolveVariant } from "./variant.ts";

/** Package lookups shared by every load in this process. */
const packages = new PackageLocator();

/** Inject Destack module metadata while Bun loads package sources, resolving server variants. */
export const modulePlugin: Bun.BunPlugin = {
    name: "destack-module",
    setup(build) {
        // replace relative imports with their server variants, since Bun runs server code
        build.onResolve({ filter: /^\./ }, ({ path, importer }) => {
            const variant = resolveVariant(path, importer, "server");

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

            // pass unchanged sources on to later bundler plugins
            const result = owner && transformModule(code, path, owner.metadata);
            if (!result && isBundling) {
                return undefined;
            }
            const contents = result
                ? `${result.code}\n//# sourceMappingURL=${result.map.toUrl()}`
                : code;

            return { contents, loader: /\.[cm]?tsx$/.test(path) ? "tsx" : "ts" };
        });
    },
};
