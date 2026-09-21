import type { BuildOptions } from "../../../src/index.ts";

/** Compile public modules and their declarations without bundling dependencies. */
export const request = {
    dependencies: {},
    outputs: {
        library: {
            kind: "module",
            target: "server",
            runtime: "deno",
            bundle: false,
        },
    },
} satisfies Omit<BuildOptions, "directory">;
