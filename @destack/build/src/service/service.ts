import { build } from "./build.ts";
import { preview } from "./preview.ts";
import { inspect } from "./inspect.ts";

/** Package compilation and running application previews. */
export const buildService = { inspect, build, preview };
