import type { PackageBuildOptions } from "@destack/build";

/** Distribute resource declarations. */
export const request = {
    outputs: { library: { kind: "module", runtime: "bun" } },
} satisfies Omit<PackageBuildOptions, "directory" | "dependencies">;
