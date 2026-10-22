import type { PackageBuildOptions } from "@destack/build";

/** Compile the same workload for local and worker hosts. */
export const request = {
    outputs: {
        bun: { kind: "module", runtime: "bun" },
        worker: { kind: "module", runtime: "workerd" },
    },
} satisfies Omit<PackageBuildOptions, "directory" | "dependencies">;
