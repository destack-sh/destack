import type { BuildOptions } from "../../../src/index.ts";

/** Compile the same service for local and worker hosts. */
export const request = {
    outputs: {
        bun: { kind: "module", runtime: "bun", bundle: true },
        worker: { kind: "module", runtime: "workerd", bundle: true },
    },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
