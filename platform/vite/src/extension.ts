import type { Plugin } from "vite";
import { createFilter } from "vite";

/// Destack & Destack Vite plugin.
export default function destackPlugin(opts?: {
    include?: string | RegExp | (string | RegExp)[];
    exclude?: string | RegExp | (string | RegExp)[];
}): Plugin {
    const filter = createFilter(opts?.include ?? /\.(ds|d\.ds)$/, opts?.exclude);

    return {
        name: "destack",
        enforce: "pre",

        async load(id) {
            return null;
        },

        async transform(code, id, options) {
            if (!filter(id)) return null;

            // TODO #Incomplete: implement Vite plugin
            throw new Error("not implemented");
        },

        handleHotUpdate(ctx) {
            return;
        },
    };
}
