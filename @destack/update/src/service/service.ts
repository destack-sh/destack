import { defineService } from "@destack/service";
import { distribution } from "../object/index.ts";
import type {} from "@destack/package/import-meta";

/** The distribution of a machine: the release installed, the one staged, and the restart activating it. */
export const distributionService = defineService("distribution", {
    objects: { distribution },
});
