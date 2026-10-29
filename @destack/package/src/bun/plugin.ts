/// <reference types="bun" />
import type { FileSystem } from "@destack/environment";
import { PackageLocator } from "../locator/index.ts";
import { DECLARED_MODULE, DECLARED_PASSTHROUGH, transformModule } from "../transform/transform.ts";

/** Inject Destack module metadata while Bun loads package sources, read from files. */
export function modulePlugin(files: FileSystem): Bun.BunPlugin {
    // look packages up once for every load through the plugin
    const packages = new PackageLocator(files);

    return {
        name: "destack-module",
        setup(build) {
            // provide the declaration recorder as its pass-through
            build.module(DECLARED_MODULE, () => ({ contents: DECLARED_PASSTHROUGH, loader: "js" }));

            // stamp package sources, since runtime loads need contents and bundles take the rest
            const isBundling = build.config !== undefined;
            build.onLoad({ filter: /\.[cm]?tsx?$/u }, async ({ path }) => {
                // read the source of a Destack package's module
                const owner = await packages.find(path);
                const code = await files.readTextFile(path);

                // pass unchanged sources on to later bundler plugins
                const result = owner && (await transformModule(code, path, owner, packages));
                if (!result && isBundling) {
                    return undefined;
                }
                const loader = /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts";
                if (!result) {
                    return { contents: code, loader };
                }

                // map stamped sources back to their authored lines
                const url = result.map.toUrl();

                return { contents: `${result.code}\n//# sourceMappingURL=${url}`, loader };
            });
        },
    };
}
