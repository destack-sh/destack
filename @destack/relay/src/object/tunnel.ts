import { none } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A machine's tunnel to the relay, global as the machine is, written by the relay alone. */
export const tunnel = defineObject({
    name: "tunnel",
    plural: "tunnels",
    scope: "universe",
    isScope: true,
    partition: true,
    key: schema.identifier("tunnel"),
    fields: {
        /** The name the relay routes to the machine, which its connections answer renewals with. */
        name: field.string(schema.string().min(1)),
        /** The publication of the machine's endpoint at the relay while a connection is open. */
        publication: field.string(schema.string().min(1)).optional(),
    },
    permissions: { read: none() },
}).methods();

/** One WebSocket connection of a machine's tunnel. */
export const connection = defineObject({
    name: "connection",
    plural: "connections",
    scope: tunnel,
    key: schema.identifier("connection"),
    fields: {
        /** When the connection's token lapses, in UTC epoch milliseconds. */
        lapsesAt: field.time(),
    },
    permissions: { read: none() },
}).methods();
