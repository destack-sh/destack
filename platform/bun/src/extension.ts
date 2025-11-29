import fs from "node:fs";
import path from "node:path";
import {
    defaultTranspileOptions,
    TranspileTarget,
    transpileFile,
    Workspace,
    type TranspileOptions,
} from "@destack/napi";
import type { BunPlugin, PluginBuilder } from "bun";

/// Options for the Destack Bun plugin.
export interface DestackPluginOptions {
    /// The transpile options to use.
    transpileOptions?: Partial<TranspileOptions>;
}

/// Destack Bun plugin for transpiling files.
export const destackPlugin = (options: DestackPluginOptions = {}): BunPlugin => ({
    name: "destack",
    setup(build: PluginBuilder) {
        // merge user options with defaults
        const transpileOptions: TranspileOptions = {
            ...defaultTranspileOptions(),
            target: TranspileTarget.TypeScript,
            ...options.transpileOptions,
        };

        // create "persistent" workspace for resolution
        const workspace = new Workspace({
            cwd: process.cwd(),
        });
        workspace.addRoot(process.cwd());

        // resolve extensionless imports (`.ds`, `.d.ds`, `index.ds`, or `index.d.ds`)
        build.onResolve({ filter: /^[^.].*$|^\.\.?($|\/)/ }, (args) => {
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

            // find the Destack file
            const dsPath = candidates.find((p) => fs.existsSync(p));
            if (dsPath != null) {
                return { path: dsPath };
            }
            // delegate back to Bun for normal files
            return;
        });

        // load .ds and .d.ds files
        build.onLoad({ filter: /\.(ds|d\.ds)$/ }, async (args: { path: string }) => {
            // read file content
            const content = await fs.promises.readFile(args.path, "utf-8");

            // transpile to TypeScript
            const result = transpileFile(args.path, content, transpileOptions);

            return {
                contents: result.code,
                loader: "ts",
            };
        });
    },
});

// default export for convenience
export default destackPlugin;
