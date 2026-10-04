import type { Plugin } from "vite";
import { PackageLocator } from "../transform/locator.ts";
import { transformModule } from "../transform/transform.ts";

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
            filter: { id: /\.[cm]?tsx?$/u },
            async handler(code, id) {
                // leave virtual modules and files outside Destack packages unchanged
                const query = id.indexOf("?");
                const path = query === -1 ? id : id.slice(0, query);
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
