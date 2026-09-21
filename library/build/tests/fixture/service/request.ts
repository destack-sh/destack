import type { BuildOptions } from "../../../src/index.ts";

/** Compile the same service for local and worker hosts. */
export const request = {
    outputs: {
        bun: { kind: "module", target: "server", runtime: "bun", bundle: true },
        worker: { kind: "module", target: "server", runtime: "workerd", bundle: true },
    },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
