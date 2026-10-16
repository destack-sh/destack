import { none, through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { tunnel } from "./tunnel.ts";

/** One WebSocket connection of a machine's tunnel. */
export const connection = defineObject(
    {
        name: "connection",
        plural: "connections",
        scope: tunnel,
        fields: {
            /** When the connection's token lapses, in UTC epoch milliseconds. */
            lapsesAt: field.time(),
        },
        // let the tunnel's serving installation record its connections
        permissions: { read: none(), connect: through("scope", "connect") },
    },
    (method) => ({
        /** Record a connection the tunnel accepted, until its token lapses. */
        create: method.create("connect", { fields: ["lapsesAt"], isPredicted: false }),
        /** Move a connection's lapse to its renewed token's. */
        renew: method.update("connect", { fields: ["lapsesAt"], isPredicted: false }),
        /** Forget a connection once it closed or lapsed. */
        delete: method.delete("connect", { isPredicted: false }),
    }),
);
