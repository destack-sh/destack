import type { PackageBuildOptions } from "@destack/build";

/** Compile a stack that imports declarations from another source package. */
export const request = {
    outputs: { stack: { kind: "module", runtime: "bun" } },
} satisfies Omit<PackageBuildOptions, "directory" | "dependencies">;
