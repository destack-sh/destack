import type { BuildExtension } from "@destack/package/build";
import { fontPlugin } from "./font.ts";
import { webOutput } from "./output.ts";

/** Compile `web` outputs, Solid applications with a browser output and an optional server output, and read imported fonts. */
export const webExtension: BuildExtension = {
    transform: () => [fontPlugin()],
    outputs: { web: webOutput },
};
