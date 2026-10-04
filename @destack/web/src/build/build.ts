import type { BuildExtension } from "@destack/package/build";
import { webOutput } from "./output.ts";

/** Compile `web` outputs: Solid applications with a browser output and an optional server output. */
export const webExtension: BuildExtension = {
    outputs: { web: webOutput },
};
