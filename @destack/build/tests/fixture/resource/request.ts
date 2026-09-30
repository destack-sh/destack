import type { BuildOptions } from "../../../src/index.ts";

/** Distribute resource declarations. */
export const request = {
    outputs: { library: { kind: "module", runtime: "bun", bundle: true } },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
