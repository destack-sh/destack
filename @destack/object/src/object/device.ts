import { contained, principal } from "@destack/access";
import { defineObject } from "./object.ts";

/** The program instances on computers, the scope each device's local objects live in. */
export const device = defineObject({
    name: "device",
    plural: "devices",
    scope: "universe",
    represents: principal.device,
    fields: {},
    permissions: {
        // let the people signed in on the device, who live inside it, see and use its local objects
        read: contained(principal.user),
        use: contained(principal.user),
    },
});
