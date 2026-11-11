import { contained, principal } from "@destack/access";
import { defineObject } from "./object.ts";
import { argument, plural, t } from "@destack/locale";

/** The program instances on computers, the scope each device's local objects live in. */
export const device = defineObject({
    name: "device",
    plural: "devices",
    title: t`${plural(argument("count"), { one: "Device", other: "Devices" })}`,
    scope: "universe",
    fields: {},
    access: {
        represents: principal.device,
        permissions: {
            // let the device and the people signed in on it, who live inside it, see and use its local objects
            read: contained(principal.user, principal.device),
            use: contained(principal.user, principal.device),
            // let the device replicate its chain
            replicate: contained(principal.device),
        },
    },
});
