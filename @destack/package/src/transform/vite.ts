import type { Plugin } from "vite";
import { PackageLocator, transformModule } from "./transform.ts";
import { requireBase, resolveVariant } from "./variant.ts";

/**
 * Inject Destack module metadata while Vite loads package sources.
 *
 * Each Vite environment resolves the variants of its target: browser for clients, else server.
 * Bundles outside Vite environments, like declaration evaluation, load the bases.
 */
export function modulePlugin(): Plugin {
    const packages = new PackageLocator();

    return {
        name: "destack-module",
        enforce: "pre",
        resolveId: {
            filter: { id: /^\./ },
            handler(specifier, importer) {
                // replace a relative import with the environment's variant when one exists
                const consumer = this.environment?.config.consumer;
                if (consumer === undefined || importer === undefined || importer.startsWith("\0")) {
                    return;
                }
                const target = consumer === "client" ? "browser" : "server";

                return resolveVariant(specifier, importer.split("?")[0]!, target);
            },
        },
        transform: {
            filter: { id: /\.[cm]?tsx?$/ },
            async handler(code, id) {
                // leave virtual modules and files outside Destack packages unchanged
                const path = id.split("?")[0]!;
                if (id.startsWith("\0")) {
                    return;
                }
                const owner = await packages.find(path);
                if (!owner) {
                    return;
                }

                // require variants to re-export their base, then stamp declarations
                requireBase(code, path);

                return transformModule(code, path, owner.metadata);
            },
        },
    };
}
