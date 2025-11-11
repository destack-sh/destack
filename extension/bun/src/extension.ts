import fs from "node:fs";
import path from "node:path";
import type { BunPlugin, PluginBuilder } from "bun";

/// Destack & Dyst Bun plugin.
export const destackPlugin: BunPlugin = {
    name: "destack",
    setup(build: PluginBuilder) {
        // ensure the compiler is running
        // ...

        // resolve extensionless (`.ds`, `.d.ds`, `index.ds`, or `index.d.ds`)
        build.onResolve({ filter: /^[^.].*$|^\.\.?($|\/)/ }, (args) => {
            // only handle relative or absolute specifiers without an explicit extension
            if (!args.path.startsWith(".") && !path.isAbsolute(args.path)) {
                return; // let Bun resolve bare specifiers/packages
            }

            // build the candidate paths
            const base = path.isAbsolute(args.path)
                ? args.path
                : path.join(args.resolveDir, args.path);
            const candidates = [
                `${base}.ds`,
                `${base}.d.ds`,
                path.join(base, "index.ds"),
                path.join(base, "index.d.ds"),
            ];

            // find the Dyst file
            const dsPath = candidates.find((p) => fs.existsSync(p));
            if (dsPath != null) {
                return { path: dsPath };
            } else {
                return; // delegate back to Bun for normal files
            }
        });

        // load .ds and .d.ds files
        build.onLoad({ filter: /\.(ds|d\.ds)$/ }, async (args: { path: string }) => {
            // nocheckin: Bun plugin
            return {
                contents: `console.log("${args.path}")`,
                loader: "ts",
            };
        });
    },
};
