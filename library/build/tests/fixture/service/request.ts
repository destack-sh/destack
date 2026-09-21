import type { BuildOptions } from "../../../src/index.ts";

/** Compile the same service for local and worker hosts. */
export const request = {
    outputs: {
        deno: { kind: "module", target: "server", runtime: "deno", bundle: true },
        worker: { kind: "module", target: "server", runtime: "workerd", bundle: true },
    },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
