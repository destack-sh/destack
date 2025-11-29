import {
    defaultTranspileOptions,
    type TranspileOptions,
    TranspileTarget,
    transpileFile,
    Workspace,
} from "@destack/napi";
import type { HmrContext, Plugin } from "vite";
import { createFilter } from "vite";

/// Options for the Destack Vite plugin.
export interface DestackPluginOptions {
    /// File patterns to include.
    include?: string | RegExp | (string | RegExp)[];
    /// File patterns to exclude.
    exclude?: string | RegExp | (string | RegExp)[];
    /// The transpile options to use.
    transpileOptions?: Partial<TranspileOptions>;
}

/// Destack Vite plugin for transpiling files.
export default function destackPlugin(opts: DestackPluginOptions = {}): Plugin {
    const filter = createFilter(opts.include ?? /\.(ds|d\.ds)$/, opts.exclude);

    // merge user options with defaults
    const transpileOptions: TranspileOptions = {
        ...defaultTranspileOptions(),
        target: TranspileTarget.TypeScript,
        ...opts.transpileOptions,
    };

    // create "persistent" workspace for resolution
    let workspace: Workspace | null = null;

    return {
        name: "destack",
        enforce: "pre",

        // initialize workspace on config resolution
        configResolved(config) {
            workspace = new Workspace({
                cwd: config.root,
            });
            workspace.addRoot(config.root);
        },

        // transform .ds files to TypeScript
        async transform(code: string, id: string) {
            if (!filter(id)) return null;

            try {
                const result = transpileFile(id, code, transpileOptions);
                return {
                    code: result.code,
                    map: null,
                };
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                this.error(`failed to transpile ${id}: ${message}`);
            }
        },

        // handle HMR for .ds files
        handleHotUpdate(ctx: HmrContext) {
            // only handle .ds files
            if (!filter(ctx.file)) return;

            // invalidate the module and its importers
            const modules = ctx.modules;
            return modules;
        },
    };
}
