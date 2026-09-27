import type { BuildOptions } from "../../../src/index.ts";

/** Distribute resource declarations. */
export const request = {
    outputs: { library: { kind: "module", target: "server", runtime: "bun", bundle: true } },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
