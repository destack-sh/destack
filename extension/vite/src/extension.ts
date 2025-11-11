import type { Plugin } from "vite";
import { createFilter } from "vite";

/// Destack & Dyst Vite plugin.
export default function destackPlugin(opts?: {
    include?: string | RegExp | (string | RegExp)[];
    exclude?: string | RegExp | (string | RegExp)[];
}): Plugin {
    const filter = createFilter(opts?.include ?? [/\.ds$/, /\.d.ds$/], opts?.exclude);

    return {
        name: "destack",
        enforce: "pre",

        resolveId(id, importer) {
            if (!id.includes(".") && importer) {
                const withExt = `${id}.ds`;
                return this.resolve(withExt, importer, { skipSelf: true });
            }
            return null;
        },

        async load(id) {
            // optionally read files yourself for virtual/remote sources
            return null;
        },

        async transform(code, id, options) {
            if (!filter(id)) return null;

            // nocheckin: Vite plugin
            return null;
        },

        handleHotUpdate(ctx) {
            return;
        },
    };
}
