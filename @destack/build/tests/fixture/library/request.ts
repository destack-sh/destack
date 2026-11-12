import type { PackageBuildOptions } from "../../../src/index.ts";

/** Compile public modules and their declarations, their dependencies external. */
export const request = {
    dependencies: {},
    links: {},
    outputs: {
        library: {
            kind: "module",
            runtime: "bun",
        },
    },
} satisfies Omit<PackageBuildOptions, "directory">;
