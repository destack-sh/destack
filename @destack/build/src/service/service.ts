import { defineService } from "@destack/service";
import { build } from "./build.ts";
import { preview } from "./preview.ts";
import { inspect } from "./inspect.ts";

/** Package compilation and running application previews. */
export const buildService = defineService("build", { inspect, build, preview });
