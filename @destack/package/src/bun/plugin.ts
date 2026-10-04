/// <reference types="bun" />
import { readFile } from "node:fs/promises";
import { PackageLocator } from "../transform/locator.ts";
import { transformModule } from "../transform/transform.ts";

/** Package lookups shared by every load in this process. */
const packages = new PackageLocator();

/** Inject Destack module metadata while Bun loads package sources. */
export const modulePlugin: Bun.BunPlugin = {
    name: "destack-module",
    setup(build) {
        // stamp package sources, since runtime loads need contents and bundles take the rest
        const isBundling = build.config !== undefined;
        build.onLoad({ filter: /\.[cm]?tsx?$/u }, async ({ path }) => {
            // read the source of a Destack package's module
            const owner = await packages.find(path);
            const code = await readFile(path, "utf8");

            // pass unchanged sources on to later bundler plugins
            const result = owner && transformModule(code, path, owner, packages);
            if (!result && isBundling) {
                return undefined;
            }
            const loader = /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts";
            if (!result) {
                return { contents: code, loader };
            }

            // map stamped sources back to their authored lines
            const map = result.source.generateMap({ source: path });

            return { contents: `${result.code}\n//# sourceMappingURL=${map.toUrl()}`, loader };
        });
    },
};
