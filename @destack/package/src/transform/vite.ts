import type { Plugin } from "vite";
import { PackageLocator } from "./locator.ts";
import { transformModule } from "./transform.ts";

/** Inject Destack module metadata while Vite loads package sources. */
export function modulePlugin(): Plugin {
    let packages = new PackageLocator();

    return {
        name: "destack-module",
        enforce: "pre",
        buildStart() {
            // read package definitions afresh for each build of a warm builder
            packages = new PackageLocator();
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

                // stamp declarations with a full map
                const result = transformModule(code, path, owner, packages);

                return (
                    result && {
                        code: result.code,
                        map: result.source.generateMap({ source: path, hires: true }),
                    }
                );
            },
        },
    };
}
