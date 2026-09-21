import type { BuildOptions } from "../../../src/index.ts";

/** Compile a stack that imports declarations from another source package. */
export const request = {
    outputs: { stack: { kind: "module", target: "server", runtime: "deno", bundle: true } },
} satisfies Omit<BuildOptions, "directory" | "dependencies">;
