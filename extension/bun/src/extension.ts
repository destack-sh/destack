import fs from "node:fs";
import path from "node:path";
import {
    defaultTranspilerOptions,
    TranspilerLanguage,
    type TranspilerOptions,
    TranspilerTarget,
} from "@destack/napi";
import type { BunPlugin, PluginBuilder } from "bun";

// nocheckin: bun plugin
class Transpiler {
    constructor(private readonly options: TranspilerOptions) {
        this.options = options;
    }

    transpile() {}

    getTranspiled(path: string, language: TranspilerLanguage): string | null {
        return null;
    }
}

/// Destack & Dyst Bun plugin.
export const destackPlugin: BunPlugin = {
    name: "destack",
    setup(build: PluginBuilder) {
        console.debug("setup");
        
        // prepare the transpiler
        let transpiler = new Transpiler({
            ...defaultTranspilerOptions(),
            target: TranspilerTarget.TypeScript,
        });

        // re-transpile everything on start
        // TODO #Incomplete: support HMR properly
        build.onStart(() => {
            console.debug("onStart");
            transpiler = new Transpiler({
                ...defaultTranspilerOptions(),
                target: TranspilerTarget.TypeScript,
            });
            transpiler.transpile();
        });

        // resolve extensionless (`.ds`, `.d.ds`, `index.ds`, or `index.d.ds`)
        build.onResolve({ filter: /^[^.].*$|^\.\.?($|\/)/ }, (args) => {
            console.debug("onResolve", args.path);
            // only handle relative or absolute specifiers without an explicit extension
            if (!args.path.startsWith(".") && !path.isAbsolute(args.path)) {
                return; // let Bun resolve bare specifiers/packages
            }

            // build the candidate paths
            const resolveDir =
                args.resolveDir ??
                (args.importer.length > 0 ? path.dirname(args.importer) : process.cwd());
            const base = path.isAbsolute(args.path) ? args.path : path.join(resolveDir, args.path);
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
            console.debug("onLoad", args.path);
            const content = transpiler.getTranspiled(args.path, TranspilerLanguage.TypeScript);
            return {
                contents: content ?? "",
                loader: "ts",
            };
        });
    },
};
