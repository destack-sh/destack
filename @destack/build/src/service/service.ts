import { defineService } from "@destack/service";
import { build } from "./build.ts";
import { inspect } from "./inspect.ts";

/** Package inspection and compilation. */
export const buildService = defineService("build", { inspect, build });
