import type { PackageBuildOptions } from "../../../src/index.ts";

/** Compile public modules and their declarations without bundling dependencies. */
export const request = {
    dependencies: {},
    outputs: {
        library: {
            kind: "module",
            runtime: "bun",
            bundle: false,
        },
    },
} satisfies Omit<PackageBuildOptions, "directory">;
