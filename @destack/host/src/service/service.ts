import { defineService } from "@destack/service";
import { host, hostKey } from "../object/index.ts";
import { token } from "./token.ts";

/** Hosts and their keys, served as objects in the global tier, and the access tokens hosts are granted. */
export const hostService = defineService("host", {
    token,
    objects: { host, hostKey },
});
