import { defineService } from "@destack/service";
import { distribution } from "../object/index.ts";
import type {} from "@destack/package/import-meta";

/** A machine's distribution: the installed release, the staged one, and the restart into it. */
export const distributionService = defineService("distribution", {
    objects: { distribution },
});
