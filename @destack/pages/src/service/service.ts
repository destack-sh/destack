import { defineService } from "@destack/service";
import { page } from "../object/index.ts";

/** Pages, their trees, sharing, links, trash and sync. */
export const pagesService = defineService("pages", {
    objects: { page },
});
