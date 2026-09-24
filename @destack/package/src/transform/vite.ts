import type { Plugin } from "vite";
import { ModulePackages, transformModule } from "./transform.ts";

/** Inject Destack module metadata while Vite loads package sources. */
export function modulePlugin(): Plugin {
    const packages = new ModulePackages();

    return {
        name: "destack-module",
        enforce: "pre",
        transform: {
            filter: { id: /\.[cm]?tsx?$/ },
            async handler(code, id) {
                // leave virtual modules and files outside Destack packages unchanged
                const path = id.split("?")[0];
                if (id.startsWith("\0")) {
                    return;
                }
                const owner = await packages.find(path);

                return owner ? transformModule(code, path, owner.metadata) : undefined;
            },
        },
    };
}
