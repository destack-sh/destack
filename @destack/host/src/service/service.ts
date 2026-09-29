import { defineService } from "@destack/service";
import { host, hostKey } from "../object/index.ts";

/** Hosts and their keys, served as objects in the global tier. */
export const hostService = defineService("host", {
    objects: { host, hostKey },
});
