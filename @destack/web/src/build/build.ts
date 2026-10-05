import type { BuildExtension } from "@destack/package/build";
import { fontPlugin } from "./font.ts";
import { imagePlugin } from "./image.ts";
import { webOutput } from "./output.ts";

/** Compile `web` outputs, Solid applications with a browser output and an optional server output, and turn imported images and fonts into assets. */
export const webExtension: BuildExtension = {
    transform: () => [imagePlugin(), fontPlugin()],
    outputs: { web: webOutput },
};
