/// <reference types="bun" />
import { readFile } from "node:fs/promises";
import { ModulePackages, transformModule } from "./transform.ts";

/** Package lookups shared by every load in this process. */
const packages = new ModulePackages();

/** Inject Destack module metadata while Bun loads or bundles package sources. */
export const modulePlugin: Bun.BunPlugin = {
    name: "destack-module",
    setup(build) {
        const isBundling = build.config !== undefined;
        build.onLoad({ filter: /\.[cm]?tsx?$/ }, async ({ path }) => {
            // leave files outside Destack packages to the default loader
            const owner = await packages.find(path);
            if (!owner) {
                return undefined;
            }

            // leave unchanged sources to later bundler plugins; runtime loads require contents
            const code = await readFile(path, "utf8");
            const result = transformModule(code, path, owner.metadata);
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
